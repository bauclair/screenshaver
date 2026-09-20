// Import Data wizard UI.
//
// Checkpoint: Select Archive plus read-only Inspect Archive.
// Inspection never writes the database, managed shader folder, configuration,
// or invokes shader compilation/preprocessing/rendering.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use rusqlite::OptionalExtension;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImportStage {
    SelectArchive,
    InspectArchive,
    ResolveConflicts,
    Review,
    Results,
}

impl ImportStage {
    const ALL: [Self; 5] = [
        Self::SelectArchive,
        Self::InspectArchive,
        Self::ResolveConflicts,
        Self::Review,
        Self::Results,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::SelectArchive => "Select Archive",
            Self::InspectArchive => "Inspect Archive",
            Self::ResolveConflicts => "Resolve Conflicts",
            Self::Review => "Review & Confirm",
            Self::Results => "Results",
        }
    }
}

#[derive(Clone, Debug)]
struct ImportWizardState {
    open: bool,
    stage: ImportStage,
    archive_path: String,
    inspection: Option<ArchiveInspection>,
    package: Option<ValidatedPackage>,
    conflicts: Option<ConflictReport>,
    proposed_plan: Option<ProposedImportPlan>,
    import_result: Option<ImportExecutionResult>,
    resolutions: BTreeMap<(String, u64), ConflictResolution>,
    conflict_rename_stamp: Option<String>,
}

impl Default for ImportWizardState {
    fn default() -> Self {
        Self {
            open: false,
            stage: ImportStage::SelectArchive,
            archive_path: String::new(),
            inspection: None,
            package: None,
            conflicts: None,
            proposed_plan: None,
            import_result: None,
            resolutions: BTreeMap::new(),
            conflict_rename_stamp: None,
        }
    }
}

impl ImportWizardState {
    fn reset_for_open(&mut self) {
        *self = Self { open: true, ..Self::default() };
    }

    fn archive_selected(&self) -> bool {
        !self.archive_path.trim().is_empty()
    }

    fn inspection_passed(&self) -> bool {
        self.inspection.as_ref().map(|result| result.passed).unwrap_or(false)
    }

    fn stage_enabled(&self, stage: ImportStage) -> bool {
        match stage {
            ImportStage::SelectArchive => true,
            ImportStage::InspectArchive => self.archive_selected(),
            ImportStage::ResolveConflicts => self.conflicts.is_some(),
            ImportStage::Review => self.proposed_plan.is_some() && self.conflicts.is_some(),
            ImportStage::Results => self.import_result.is_some(),
        }
    }
}

#[derive(Clone, Debug)]
struct InspectionCheck {
    passed: bool,
    title: String,
    detail: String,
}

#[derive(Clone, Debug)]
struct ArchiveInspection {
    passed: bool,
    format_version: Option<u32>,
    source_version: Option<String>,
    source_db_schema: Option<u32>,
    export_focus: Option<String>,
    policies: usize,
    shaders: usize,
    playlists: usize,
    memberships: usize,
    checks: Vec<InspectionCheck>,
}

impl ArchiveInspection {
    fn new() -> Self {
        Self {
            passed: false,
            format_version: None,
            source_version: None,
            source_db_schema: None,
            export_focus: None,
            policies: 0,
            shaders: 0,
            playlists: 0,
            memberships: 0,
            checks: Vec::new(),
        }
    }

    fn pass(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.checks.push(InspectionCheck {
            passed: true,
            title: title.into(),
            detail: detail.into(),
        });
    }

    fn fail(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.checks.push(InspectionCheck {
            passed: false,
            title: title.into(),
            detail: detail.into(),
        });
    }

    fn finish(&mut self) {
        self.passed = !self.checks.is_empty()
            && self.checks.iter().all(|check| check.passed);
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
struct Manifest {
    format: String,
    format_version: u32,
    screenshaver_version: String,
    database_schema_version: u32,
    export_focus: String,
    policy_count: usize,
    shader_count: usize,
    playlist_count: usize,
    package_sha256: String,
    files: BTreeMap<String, ManifestFile>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ManifestFile {
    sha256: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SchemaArchive {
    manifest: String,
    shader_directory: String,
    metadata_files: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SchemaIntegrity {
    algorithm: String,
    package_canonicalization: String,
    package_hash_excludes: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SchemaDataset {
    file: String,
    columns: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SchemaDatasets {
    policies: SchemaDataset,
    shaders: SchemaDataset,
    playlists: SchemaDataset,
    playlist_members: SchemaDataset,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExportSchema {
    format: String,
    format_version: u32,
    archive: SchemaArchive,
    integrity: SchemaIntegrity,
    datasets: SchemaDatasets,
}

#[derive(Clone, Debug)]
struct Payload {
    bytes: Vec<u8>,
    is_dir: bool,
    unix_mode: Option<u32>,
}

#[derive(Clone, Debug)]
struct Tsv {
    rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
struct PackagePolicy {
    export_id: u64,
    name: String,
    target: String,
    shader_export_id: u64,
    values: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct PackageShader {
    export_id: u64,
    filename: String,
    archive_path: String,
    sha256: String,
}

#[derive(Clone, Debug)]
struct PackagePlaylist {
    export_id: u64,
    name: String,
    description: String,
}

#[derive(Clone, Debug)]
struct PackageMembership {
    playlist_export_id: u64,
    policy_export_id: u64,
    position: u64,
}

#[derive(Clone, Debug)]
struct ValidatedPackage {
    policies: Vec<PackagePolicy>,
    shaders: Vec<PackageShader>,
    playlists: Vec<PackagePlaylist>,
    memberships: Vec<PackageMembership>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConflictKind {
    New,
    Duplicate,
    Conflict,
}

impl ConflictKind {
    fn label(self) -> &'static str {
        match self {
            Self::New => "NEW",
            Self::Duplicate => "DUPLICATE",
            Self::Conflict => "CONFLICT",
        }
    }
}

#[derive(Clone, Debug)]
struct ConflictResolution {
    renamed_name: String,
}

#[derive(Clone, Debug)]
struct ConflictItem {
    kind: ConflictKind,
    object_type: &'static str,
    package_id: u64,
    name: String,
    detail: String,
}

#[derive(Clone, Debug, Default)]
struct ConflictReport {
    items: Vec<ConflictItem>,
    shader_local_ids: BTreeMap<u64, i64>,
    policy_local_ids: BTreeMap<u64, i64>,
    playlist_local_ids: BTreeMap<u64, i64>,
    shader_kinds: BTreeMap<u64, ConflictKind>,
    policy_kinds: BTreeMap<u64, ConflictKind>,
    playlist_kinds: BTreeMap<u64, ConflictKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProposedDestination {
    Existing(i64),
    New,
    Unresolved,
}

impl ProposedDestination {
    fn label(self, object_type: &str) -> String {
        match self {
            Self::Existing(id) => format!("Keep Existing {} ID {}", object_type, id),
            Self::New => format!("Import new {}", object_type),
            Self::Unresolved => "Unresolved conflict".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct ProposedPolicyPlan {
    package_id: u64,
    name: String,
    destination: ProposedDestination,
    shader_package_id: u64,
    shader_destination: ProposedDestination,
}

#[derive(Clone, Debug)]
struct ProposedPlaylistMember {
    position: u64,
    policy_package_id: u64,
    policy_name: String,
    policy_destination: ProposedDestination,
}

#[derive(Clone, Debug)]
struct ProposedPlaylistPlan {
    package_id: u64,
    name: String,
    destination: ProposedDestination,
    members: Vec<ProposedPlaylistMember>,
}

#[derive(Clone, Debug, Default)]
struct ProposedImportPlan {
    policies: Vec<ProposedPolicyPlan>,
    playlists: Vec<ProposedPlaylistPlan>,
    unresolved_dependencies: usize,
}

impl ConflictReport {
    fn count(&self, kind: ConflictKind) -> usize {
        self.items.iter().filter(|item| item.kind == kind).count()
    }
}

#[derive(Clone, Debug)]
struct ImportExecutionResult {
    success: bool,
    detail: String,
    backup_path: Option<PathBuf>,
    shaders_created: usize,
    policies_created: usize,
    playlists_created: usize,
    memberships_created: usize,
}

impl ImportExecutionResult {
    fn failure(detail: impl Into<String>, backup_path: Option<PathBuf>) -> Self {
        Self {
            success: false,
            detail: detail.into(),
            backup_path,
            shaders_created: 0,
            policies_created: 0,
            playlists_created: 0,
            memberships_created: 0,
        }
    }
}

const MAX_ZIP_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 4096;

fn state_id() -> egui::Id {
    egui::Id::new("screenshaver_import_data_wizard_state")
}

pub fn open(ctx: &egui::Context) {
    let id = state_id();
    ctx.data_mut(|data| {
        let mut state = data.get_temp::<ImportWizardState>(id).unwrap_or_default();
        state.reset_for_open();
        data.insert_temp(id, state);
    });
}

pub fn set_archive(ctx: &egui::Context, archive: &Path) {
    let id = state_id();
    ctx.data_mut(|data| {
        let mut state = data.get_temp::<ImportWizardState>(id).unwrap_or_default();
        if state.open {
            state.archive_path = archive.to_string_lossy().into_owned();
            state.inspection = None;
            state.package = None;
            state.conflicts = None;
            state.proposed_plan = None;
            state.import_result = None;
            state.resolutions.clear();
            state.conflict_rename_stamp = None;
            state.stage = ImportStage::SelectArchive;
            data.insert_temp(id, state);
        }
    });
}

pub fn draw(
    ctx: &egui::Context,
    archive_browse_requested: &mut Option<PathBuf>,
) {
    let id = state_id();
    let mut state = ctx.data(|data| {
        data.get_temp::<ImportWizardState>(id).unwrap_or_default()
    });

    if !state.open {
        return;
    }

    let scale = (ctx.screen_rect().height()
        / crate::editor_layout::EDIT_WINDOW_REFERENCE_DISPLAY_HEIGHT)
        .clamp(
            crate::editor_layout::EDIT_WINDOW_SCALE_MIN,
            crate::editor_layout::EDIT_WINDOW_SCALE_MAX,
        );

    let window_height =
        crate::editor_layout::EDIT_WINDOW_REFERENCE_HEIGHT_PIXELS * scale;

    egui::Window::new("Import Screenshaver Data")
        .id(egui::Id::new("screenshaver_import_data_wizard_window"))
        .collapsible(false)
        .resizable(true)
        .default_width(720.0)
        .min_width(720.0)
        .max_width(720.0)
        .default_height(window_height)
        .show(ctx, |ui| {
            let content_height = (window_height - 48.0).max(560.0);
            ui.set_min_height(content_height);
            let body_height = (content_height - 64.0).max(0.0);

            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), body_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    draw_stage_rail(ui, &mut state);
                    ui.separator();
                    ui.add_space(12.0);

                    ui.vertical(|ui| {
                        ui.set_min_width(430.0);
                        match state.stage {
                            ImportStage::SelectArchive => {
                                draw_select_archive(ui, &state, archive_browse_requested);
                            }
                            ImportStage::InspectArchive => {
                                draw_inspection(ui, &state);
                            }
                            ImportStage::ResolveConflicts => {
                                draw_conflict_report(ui, &mut state);
                            }
                            ImportStage::Review => {
                                draw_review(ui, &state);
                            }
                            ImportStage::Results => {
                                draw_results(ui, &state);
                            }
                        }
                    });
                },
            );

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            draw_navigation(ui, &mut state);
        });

    ctx.data_mut(|data| data.insert_temp(id, state));
}

fn draw_stage_rail(ui: &mut egui::Ui, state: &mut ImportWizardState) {
    ui.vertical(|ui| {
        ui.set_min_width(190.0);
        for stage in ImportStage::ALL {
            let response = ui.add_enabled(
                state.stage_enabled(stage),
                egui::SelectableLabel::new(
                    state.stage == stage,
                    egui::RichText::new(stage.label()).strong(),
                ),
            );

            if response.clicked() {
                if stage == ImportStage::InspectArchive && state.inspection.is_none() {
                    let (inspection, package) =
                        inspect_archive(Path::new(state.archive_path.trim()));
                    state.inspection = Some(inspection);
                    state.package = package;
                }
                if stage == ImportStage::ResolveConflicts && state.conflicts.is_none() {
                    if let Some(package) = state.package.as_ref() {
                        let conflicts = discover_conflicts(package);
                        state.conflict_rename_stamp = local_import_name_stamp().ok();
                        state.resolutions = state.conflict_rename_stamp.as_deref()
                            .map(|stamp| generate_automatic_conflict_resolutions(package, &conflicts, stamp))
                            .unwrap_or_default();
                        state.conflicts = Some(conflicts);
                        refresh_resolved_plan(state);
                    }
                }
                state.stage = stage;
            }
            ui.add_space(4.0);
        }
    });
}

fn draw_select_archive(
    ui: &mut egui::Ui,
    state: &ImportWizardState,
    browse: &mut Option<PathBuf>,
) {
    ui.heading("Select Archive");
    ui.add_space(8.0);
    ui.label(
        "Select a Screenshaver export archive to inspect. No Screenshaver data will be changed at this stage."
    );
    ui.add_space(16.0);

    egui::Grid::new("screenshaver_import_select_archive_grid")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 8.0))
        .show(ui, |ui| {
            ui.label("Archive:");
            ui.horizontal(|ui| {
                let mut path = state.archive_path.clone();
                ui.add(
                    egui::TextEdit::singleline(&mut path)
                        .desired_width(300.0)
                        .interactive(false),
                );
                if ui.button("Browse...").clicked() {
                    *browse = Some(starting_directory(&state.archive_path));
                }
            });
            ui.end_row();
        });

    ui.add_space(12.0);
    if state.archive_selected() {
        ui.label("The archive is ready for read-only inspection.");
    } else {
        ui.label(egui::RichText::new("No archive selected.").weak());
    }
}

fn draw_inspection(ui: &mut egui::Ui, state: &ImportWizardState) {
    ui.heading("Inspect Archive");
    ui.add_space(8.0);

    let Some(result) = state.inspection.as_ref() else {
        ui.label("Archive inspection has not been run.");
        return;
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Archive Inspection:")
                .strong()
        );
        ui.label(
            egui::RichText::new(
                if result.passed { "PASSED" } else { "FAILED" }
            )
            .color(
                if result.passed {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::RED
                }
            )
            .strong()
        );
    });

    ui.add_space(8.0);
    egui::Grid::new("screenshaver_import_inspection_summary")
        .num_columns(2)
        .spacing(egui::vec2(12.0, 4.0))
        .show(ui, |ui| {
            summary(ui, "Export Format:",
                result.format_version.map(|v| v.to_string()).unwrap_or_else(|| "Unknown".into()));
            summary(ui, "Source Screenshaver:",
                result.source_version.clone().unwrap_or_else(|| "Unknown".into()));
            summary(ui, "Source DB Schema:",
                result.source_db_schema.map(|v| v.to_string()).unwrap_or_else(|| "Unknown".into()));
            summary(ui, "Export Focus:",
                result.export_focus.clone().unwrap_or_else(|| "Unknown".into()));
            summary(ui, "Contents:", format!(
                "{} policies, {} shaders, {} playlists, {} memberships",
                result.policies, result.shaders, result.playlists, result.memberships
            ));
        });

    ui.add_space(12.0);
    egui::ScrollArea::vertical()
        .id_source("screenshaver_import_inspection_checks")
        .auto_shrink([false, false])
        .max_height(ui.available_height().max(120.0))
        .show(ui, |ui| {
            for check in &result.checks {
                ui.horizontal_top(|ui| {
                    // Fixed status column. The report column receives the
                    // remaining width and wraps, so inspection text cannot
                    // enlarge the Import Wizard beyond Export's dimensions.
                    ui.allocate_ui_with_layout(
                        egui::vec2(48.0, 20.0),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            ui.label(
                                egui::RichText::new(
                                    if check.passed { "PASS" } else { "FAIL" }
                                )
                                .color(
                                    if check.passed {
                                        egui::Color32::GREEN
                                    } else {
                                        egui::Color32::RED
                                    }
                                )
                                .strong()
                            );
                        },
                    );

                    ui.add_space(8.0);

                    let report_width =
                        ui.available_width().max(120.0);

                    ui.allocate_ui_with_layout(
                        egui::vec2(report_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_max_width(report_width);

                            ui.label(
                                egui::RichText::new(&check.title)
                                    .strong()
                            );

                            if !check.detail.is_empty() {
                                ui.add(
                                    egui::Label::new(&check.detail)
                                        .wrap()
                                );
                            }
                        },
                    );
                });

                ui.add_space(6.0);
            }
        });

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Inspection is read-only. No changes have been made to Screenshaver."
        ).weak()
    );
}

fn summary(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(egui::RichText::new(label).strong());
    ui.label(value);
    ui.end_row();
}

fn content_section(ui: &mut egui::Ui, label: &str, count: usize) {
    ui.label(
        egui::RichText::new(format!("{} ({})", label, count))
            .strong()
    );
    ui.separator();
}


fn draw_conflict_report(ui: &mut egui::Ui, state: &mut ImportWizardState) {
    ui.heading("Resolve Conflicts");
    ui.add_space(8.0);
    ui.label(
        "Import is additive and non-destructive. Genuine name conflicts are resolved automatically by renaming the imported object; existing recipient objects are never modified or discarded."
    );
    ui.add_space(10.0);

    let Some(report) = state.conflicts.as_ref() else {
        ui.label("Conflict discovery has not been run.");
        return;
    };
    let items = report.items.clone();

    ui.label(egui::RichText::new(format!(
        "{} new, {} duplicates, {} conflicts",
        report.count(ConflictKind::New), report.count(ConflictKind::Duplicate), report.count(ConflictKind::Conflict),
    )).strong());
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .id_source("screenshaver_import_conflict_discovery_scroll")
        .auto_shrink([false, false])
        .max_height(ui.available_height().max(120.0))
        .show(ui, |ui| {
            for kind in [ConflictKind::Conflict, ConflictKind::Duplicate, ConflictKind::New] {
                for item in items.iter().filter(|item| item.kind == kind) {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(egui::vec2(72.0, 20.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
                            ui.label(egui::RichText::new(item.kind.label()).strong());
                        });
                        let width = ui.available_width().max(120.0);
                        ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_max_width(width);
                            ui.label(egui::RichText::new(format!("{}: {}", item.object_type, item.name)).strong());
                            ui.add(egui::Label::new(&item.detail).wrap());
                            if item.kind == ConflictKind::Conflict {
                                let key = (item.object_type.to_string(), item.package_id);
                                if let Some(resolution) = state.resolutions.get(&key) {
                                    ui.label(egui::RichText::new(format!(
                                        "Automatic resolution: Rename → {}",
                                        resolution.renamed_name
                                    )).strong());
                                } else {
                                    ui.label(egui::RichText::new("Automatic rename could not be generated; Import is blocked.").strong());
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            }

            if let Some(plan) = state.proposed_plan.as_ref() {
                ui.separator();
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Proposed Dependency Plan").strong());
                if plan.unresolved_dependencies == 0 && unresolved_conflict_count(state) == 0 {
                    ui.label("All package objects and dependencies have deterministic destinations.");
                } else {
                    ui.label(format!(
                        "{} conflict rename(s) could not be generated; {} dependency reference(s) remain unresolved.",
                        unresolved_conflict_count(state), plan.unresolved_dependencies
                    ));
                }
            }
        });

    ui.add_space(6.0);
    ui.label(egui::RichText::new(
        "This report is read-only. No database rows or shader files have been changed."
    ).weak());
}

fn unresolved_conflict_count(state: &ImportWizardState) -> usize {
    state.conflicts.as_ref().map(|report| report.items.iter().filter(|item| {
        item.kind == ConflictKind::Conflict
            && !state.resolutions.contains_key(&(item.object_type.to_string(), item.package_id))
    }).count()).unwrap_or(0)
}

fn local_import_name_stamp() -> Result<String, String> {
    let connection = crate::open_database::open()?;
    connection.query_row(
        "SELECT strftime('%Y%m%d-%H%M', 'now', 'localtime')",
        [],
        |row| row.get::<_, String>(0),
    ).map_err(|error| format!("Unable to determine local Import timestamp: {}", error))
}

fn generate_automatic_conflict_resolutions(
    package: &ValidatedPackage,
    report: &ConflictReport,
    import_stamp: &str,
) -> BTreeMap<(String, u64), ConflictResolution> {
    let mut resolutions = BTreeMap::new();
    for item in report.items.iter().filter(|item| item.kind == ConflictKind::Conflict) {
        // A synthetic receiving-installation error is not a package object and
        // therefore cannot be resolved by renaming.
        if !matches!(item.object_type, "Shader" | "Policy" | "Playlist") {
            continue;
        }
        if let Ok(name) = deterministic_import_name(package, &resolutions, item, import_stamp) {
            resolutions.insert(
                (item.object_type.to_string(), item.package_id),
                ConflictResolution { renamed_name: name },
            );
        }
    }
    resolutions
}

fn deterministic_import_name(
    package: &ValidatedPackage,
    resolutions: &BTreeMap<(String,u64), ConflictResolution>,
    item: &ConflictItem,
    import_stamp: &str,
) -> Result<String,String> {
    let connection = crate::open_database::open()?;
    let (stem, extension) = if item.object_type == "Shader" {
        let path = Path::new(&item.name);
        let ext = path.extension().and_then(|v| v.to_str()).map(|v| format!(".{}", v)).unwrap_or_default();
        let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or(&item.name).to_string();
        (stem, ext)
    } else { (item.name.clone(), String::new()) };

    for ordinal in 1..10000_u32 {
        let suffix = if ordinal == 1 {
            format!(" ({})", import_stamp)
        } else {
            format!(" ({} {})", import_stamp, ordinal)
        };
        let candidate_stem = if item.object_type == "Policy" || item.object_type == "Playlist" {
            let max_stem_chars = 128_usize.saturating_sub(suffix.chars().count());
            stem.chars().take(max_stem_chars).collect::<String>()
        } else {
            stem.clone()
        };
        let candidate = format!("{}{}{}", candidate_stem, suffix, extension);
        let key = candidate.trim().to_lowercase();
        let db_exists = match item.object_type {
            "Policy" => connection.query_row("SELECT EXISTS(SELECT 1 FROM shader_policies WHERE policy_name_key=?1)", [&key], |r| r.get::<_,i64>(0)).unwrap_or(1) != 0,
            "Playlist" => connection.query_row("SELECT EXISTS(SELECT 1 FROM playlists WHERE playlist_name_key=?1)", [&key], |r| r.get::<_,i64>(0)).unwrap_or(1) != 0,
            "Shader" => {
                let managed = crate::locate_paths::shader_dir().to_string_lossy().to_string();
                connection.query_row("SELECT EXISTS(SELECT 1 FROM shaders WHERE source_path=?1 AND lower(filename)=?2)", rusqlite::params![managed, key], |r| r.get::<_,i64>(0)).unwrap_or(1) != 0
                    || crate::locate_paths::shader_dir().join(&candidate).exists()
            }
            _ => true,
        };
        let package_exists = match item.object_type {
            "Policy" => package.policies.iter().any(|v| v.export_id != item.package_id && v.name.trim().eq_ignore_ascii_case(&candidate)),
            "Playlist" => package.playlists.iter().any(|v| v.export_id != item.package_id && v.name.trim().eq_ignore_ascii_case(&candidate)),
            "Shader" => package.shaders.iter().any(|v| v.export_id != item.package_id && v.filename.eq_ignore_ascii_case(&candidate)),
            _ => true,
        };
        let reserved = resolutions.values().any(|r| r.renamed_name.eq_ignore_ascii_case(&candidate));
        if !db_exists && !package_exists && !reserved { return Ok(candidate); }
    }
    Err(format!("Unable to generate a unique imported name for '{}'.", item.name))
}

fn resolved_package_and_report(
    package: &ValidatedPackage,
    report: &ConflictReport,
    resolutions: &BTreeMap<(String,u64), ConflictResolution>,
) -> Result<(ValidatedPackage, ConflictReport), String> {
    let mut package = package.clone();
    let mut report = report.clone();
    for item in report.items.clone().into_iter().filter(|i| i.kind == ConflictKind::Conflict) {
        if !matches!(item.object_type, "Shader" | "Policy" | "Playlist") {
            return Err(format!("{}: {}", item.name, item.detail));
        }
        let key = (item.object_type.to_string(), item.package_id);
        let resolution = resolutions.get(&key)
            .ok_or_else(|| format!("{} '{}' has no deterministic imported rename.", item.object_type, item.name))?;
        let name = resolution.renamed_name.clone();
        match item.object_type {
            "Shader" => {
                package.shaders.iter_mut().find(|v| v.export_id == item.package_id)
                    .ok_or("Missing package shader.")?.filename = name;
                report.shader_kinds.insert(item.package_id, ConflictKind::New);
                report.shader_local_ids.remove(&item.package_id);
            }
            "Policy" => {
                package.policies.iter_mut().find(|v| v.export_id == item.package_id)
                    .ok_or("Missing package policy.")?.name = name;
                report.policy_kinds.insert(item.package_id, ConflictKind::New);
                report.policy_local_ids.remove(&item.package_id);
            }
            "Playlist" => {
                package.playlists.iter_mut().find(|v| v.export_id == item.package_id)
                    .ok_or("Missing package playlist.")?.name = name;
                report.playlist_kinds.insert(item.package_id, ConflictKind::New);
                report.playlist_local_ids.remove(&item.package_id);
            }
            _ => unreachable!(),
        }
    }
    for item in &mut report.items {
        if item.kind == ConflictKind::Conflict
            && resolutions.contains_key(&(item.object_type.to_string(), item.package_id))
        {
            item.kind = ConflictKind::New;
        }
    }
    Ok((package, report))
}

fn refresh_resolved_plan(state: &mut ImportWizardState) {
    if let (Some(package), Some(report)) = (state.package.as_ref(), state.conflicts.as_ref()) {
        state.proposed_plan = match resolved_package_and_report(package, report, &state.resolutions) {
            Ok((resolved_package, resolved_report)) => Some(build_proposed_import_plan(&resolved_package, &resolved_report)),
            Err(_) => Some(build_proposed_import_plan(package, report)),
        };
    }
}

#[derive(Debug)]
struct LocalPolicyValues {
    policy_id: i64,
    shader_id: i64,
    texture_mode: Option<String>, texture_family: Option<String>, texture_primitives: Option<i64>,
    palette_mode: Option<String>, palette_color: Option<String>, rendered_fps: Option<i64>,
    animation_speed: Option<f64>, starting_offset: f64, anti_aliasing: Option<String>, dithering: Option<String>,
    color_precision: Option<String>, render_scale: Option<f64>, audiovisual_effect: String,
    bloom_intensity: f64, bloom_saturation: f64, bloom_threshold: f64, bloom_frequency_rotation: f64,
    bloom_frequency_invert: bool, invert_colors: bool, flip_horizontal: bool, flip_vertical: bool, hue_rotation: f64,
}

fn discover_conflicts(package: &ValidatedPackage) -> ConflictReport {
    match discover_conflicts_inner(package) {
        Ok(report) => report,
        Err(error) => ConflictReport {
            items: vec![ConflictItem {
                kind: ConflictKind::Conflict,
                object_type: "Receiving installation",
                package_id: 0,
                name: "Conflict discovery unavailable".to_string(),
                detail: error,
            }],
            ..ConflictReport::default()
        },
    }
}

fn discover_conflicts_inner(package: &ValidatedPackage) -> Result<ConflictReport, String> {
    let connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open screenshaver.db for read-only conflict discovery: {}", error))?;
    let managed_source = crate::locate_paths::shader_dir().to_string_lossy().to_string();
    let mut report = ConflictReport::default();

    // Shaders: content identity wins over filename identity. A matching hash is reusable
    // even under another filename; a matching filename with different content is a conflict.
    for shader in &package.shaders {
        let mut by_hash = connection.prepare(
            "SELECT shader_id, filename FROM shaders WHERE source_path = ?1 AND source_hash = ?2 ORDER BY shader_id"
        ).map_err(|e| format!("Unable to prepare shader hash lookup: {}", e))?;
        let matches = by_hash.query_map(rusqlite::params![&managed_source, shader.sha256], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| format!("Unable to query shader hash '{}': {}", shader.sha256, e))?
          .collect::<Result<Vec<_>, _>>().map_err(|e| format!("Unable to decode shader hash lookup: {}", e))?;

        if let Some((local_id, local_name)) = matches.first() {
            report.shader_local_ids.insert(shader.export_id, *local_id);
            report.shader_kinds.insert(shader.export_id, ConflictKind::Duplicate);
            report.items.push(ConflictItem {
                kind: ConflictKind::Duplicate,
                object_type: "Shader",
                package_id: shader.export_id,
                name: shader.filename.clone(),
                detail: if local_name == &shader.filename {
                    "Identical shader content is already installed under the same filename.".into()
                } else {
                    format!("Identical shader content is already installed as '{}'; the existing physical shader will be kept.", local_name)
                },
            });
            continue;
        }

        let same_name: Option<(i64, Option<String>)> = connection.query_row(
            "SELECT shader_id, source_hash FROM shaders WHERE source_path = ?1 AND filename = ?2 ORDER BY shader_id LIMIT 1",
            rusqlite::params![&managed_source, shader.filename],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|e| format!("Unable to check shader filename '{}': {}", shader.filename, e))?;

        let kind = if same_name.is_some() { ConflictKind::Conflict } else { ConflictKind::New };
        if let Some((local_id, _)) = same_name { report.shader_local_ids.insert(shader.export_id, local_id); }
        report.shader_kinds.insert(shader.export_id, kind);
        report.items.push(if kind == ConflictKind::Conflict {
            ConflictItem { kind, object_type: "Shader", package_id: shader.export_id, name: shader.filename.clone(),
                detail: "The filename already exists in the managed shader inventory, but its content differs from the imported shader.".into() }
        } else {
            ConflictItem { kind, object_type: "Shader", package_id: shader.export_id, name: shader.filename.clone(),
                detail: "No installed shader has this content or managed filename.".into() }
        });
    }

    // Policies: names are compared using the same trim/lowercase key convention used by
    // the policy manager, within the exported target. An exact policy requires the same
    // resolved local shader and the same persisted rendering values.
    for policy in &package.policies {
        let key = policy.name.trim().to_lowercase();
        let mut stmt = connection.prepare(
            "SELECT policy_id, shader_id, texture_mode, texture_family, texture_primitives, palette_mode, palette_color, rendered_fps, animation_speed, starting_offset, anti_aliasing, dithering, color_precision, render_scale, audiovisual_effect, bloom_intensity, bloom_saturation, bloom_threshold, bloom_frequency_rotation, bloom_frequency_invert, invert_colors, flip_horizontal, flip_vertical, hue_rotation FROM shader_policies WHERE policy_name_key = ?1 AND policy_target = ?2 ORDER BY policy_id"
        ).map_err(|e| format!("Unable to prepare Policy Name lookup: {}", e))?;
        let rows = stmt.query_map(rusqlite::params![key, policy.target], |r| Ok(LocalPolicyValues {
            policy_id:r.get(0)?, shader_id:r.get(1)?, texture_mode:r.get(2)?, texture_family:r.get(3)?, texture_primitives:r.get(4)?, palette_mode:r.get(5)?, palette_color:r.get(6)?, rendered_fps:r.get(7)?, animation_speed:r.get(8)?, starting_offset:r.get(9)?, anti_aliasing:r.get(10)?, dithering:r.get(11)?, color_precision:r.get(12)?, render_scale:r.get(13)?, audiovisual_effect:r.get(14)?, bloom_intensity:r.get(15)?, bloom_saturation:r.get(16)?, bloom_threshold:r.get(17)?, bloom_frequency_rotation:r.get(18)?, bloom_frequency_invert:r.get::<_,i64>(19)?!=0, invert_colors:r.get::<_,i64>(20)?!=0, flip_horizontal:r.get::<_,i64>(21)?!=0, flip_vertical:r.get::<_,i64>(22)?!=0, hue_rotation:r.get(23)?
        })).map_err(|e| format!("Unable to query Policy Name '{}': {}", policy.name, e))?
          .collect::<Result<Vec<_>,_>>().map_err(|e| format!("Unable to decode Policy Name lookup: {}", e))?;

        if rows.is_empty() {
            report.policy_kinds.insert(policy.export_id, ConflictKind::New);
            report.items.push(ConflictItem { kind: ConflictKind::New, object_type:"Policy", package_id: policy.export_id, name:policy.name.clone(), detail:format!("No {} policy with this Policy Name exists.", policy.target) });
        } else if rows.len() == 1 {
            let local = &rows[0];
            let expected_shader = report.shader_local_ids.get(&policy.shader_export_id).copied();
            if expected_shader == Some(local.shader_id) && policy_values_match(policy, local)? {
                report.policy_local_ids.insert(policy.export_id, local.policy_id);
                report.policy_kinds.insert(policy.export_id, ConflictKind::Duplicate);
                report.items.push(ConflictItem { kind:ConflictKind::Duplicate, object_type:"Policy", package_id: policy.export_id, name:policy.name.clone(), detail:"An existing policy has the same target, shader content, and rendering configuration.".into() });
            } else {
                report.policy_local_ids.insert(policy.export_id, local.policy_id);
                report.policy_kinds.insert(policy.export_id, ConflictKind::Conflict);
                report.items.push(ConflictItem { kind:ConflictKind::Conflict, object_type:"Policy", package_id: policy.export_id, name:policy.name.clone(), detail:"The Policy Name already exists for this target, but its shader or rendering configuration differs.".into() });
            }
        } else {
            report.policy_kinds.insert(policy.export_id, ConflictKind::Conflict);
            report.items.push(ConflictItem { kind:ConflictKind::Conflict, object_type:"Policy", package_id: policy.export_id, name:policy.name.clone(), detail:"More than one receiving policy unexpectedly matches this Policy Name and target.".into() });
        }
    }

    // Playlists can be identical only when the receiving playlist has the same description
    // and every imported member resolves to the same existing policy in the same order.
    for playlist in &package.playlists {
        let key = playlist.name.trim().to_lowercase();
        let found: Option<(i64, Option<String>)> = connection.query_row(
            "SELECT playlist_id, description FROM playlists WHERE playlist_name_key = ?1 ORDER BY playlist_id LIMIT 1",
            [key], |r| Ok((r.get(0)?, r.get(1)?))
        ).optional().map_err(|e| format!("Unable to query playlist '{}': {}", playlist.name, e))?;
        let Some((playlist_id, local_description)) = found else {
            report.playlist_kinds.insert(playlist.export_id, ConflictKind::New);
            report.items.push(ConflictItem { kind:ConflictKind::New, object_type:"Playlist", package_id: playlist.export_id, name:playlist.name.clone(), detail:"No receiving playlist has this name.".into() });
            continue;
        };

        let imported_members = package.memberships.iter().filter(|m| m.playlist_export_id == playlist.export_id).collect::<Vec<_>>();
        let all_resolved = imported_members.iter().all(|m| report.policy_local_ids.contains_key(&m.policy_export_id));
        let mut stmt = connection.prepare("SELECT policy_id FROM playlist_members WHERE playlist_id = ?1 ORDER BY position, policy_id")
            .map_err(|e| format!("Unable to prepare playlist member lookup: {}", e))?;
        let local_members = stmt.query_map([playlist_id], |r| r.get::<_,i64>(0))
            .map_err(|e| format!("Unable to query playlist members: {}", e))?
            .collect::<Result<Vec<_>,_>>().map_err(|e| format!("Unable to decode playlist members: {}", e))?;
        let expected_members = imported_members.iter().filter_map(|m| report.policy_local_ids.get(&m.policy_export_id).copied()).collect::<Vec<_>>();
        let imported_description = if playlist.description.trim().is_empty() { None } else { Some(playlist.description.trim()) };
        let local_description_trimmed = local_description.as_deref().map(str::trim).filter(|v| !v.is_empty());

        if all_resolved && expected_members == local_members && imported_description == local_description_trimmed {
            report.playlist_local_ids.insert(playlist.export_id, playlist_id);
            report.playlist_kinds.insert(playlist.export_id, ConflictKind::Duplicate);
            report.items.push(ConflictItem { kind:ConflictKind::Duplicate, object_type:"Playlist", package_id: playlist.export_id, name:playlist.name.clone(), detail:"An existing playlist has the same description and resolved policy membership in the same canonical order.".into() });
        } else {
            report.playlist_local_ids.insert(playlist.export_id, playlist_id);
            report.playlist_kinds.insert(playlist.export_id, ConflictKind::Conflict);
            report.items.push(ConflictItem { kind:ConflictKind::Conflict, object_type:"Playlist", package_id: playlist.export_id, name:playlist.name.clone(), detail:"The playlist name already exists, but its description or resolved membership differs.".into() });
        }
    }

    Ok(report)
}

fn proposed_destination(
    package_id: u64,
    kinds: &BTreeMap<u64, ConflictKind>,
    local_ids: &BTreeMap<u64, i64>,
) -> ProposedDestination {
    match kinds.get(&package_id).copied() {
        Some(ConflictKind::Duplicate) => local_ids.get(&package_id)
            .copied()
            .map(ProposedDestination::Existing)
            .unwrap_or(ProposedDestination::Unresolved),
        Some(ConflictKind::New) => ProposedDestination::New,
        Some(ConflictKind::Conflict) | None => ProposedDestination::Unresolved,
    }
}

fn build_proposed_import_plan(
    package: &ValidatedPackage,
    conflicts: &ConflictReport,
) -> ProposedImportPlan {
    let mut plan = ProposedImportPlan::default();

    for policy in &package.policies {
        let destination = proposed_destination(
            policy.export_id,
            &conflicts.policy_kinds,
            &conflicts.policy_local_ids,
        );
        let shader_destination = proposed_destination(
            policy.shader_export_id,
            &conflicts.shader_kinds,
            &conflicts.shader_local_ids,
        );
        if shader_destination == ProposedDestination::Unresolved {
            plan.unresolved_dependencies += 1;
        }
        plan.policies.push(ProposedPolicyPlan {
            package_id: policy.export_id,
            name: policy.name.clone(),
            destination,
            shader_package_id: policy.shader_export_id,
            shader_destination,
        });
    }

    for playlist in &package.playlists {
        let destination = proposed_destination(
            playlist.export_id,
            &conflicts.playlist_kinds,
            &conflicts.playlist_local_ids,
        );
        let mut members = Vec::new();
        for membership in package.memberships.iter()
            .filter(|member| member.playlist_export_id == playlist.export_id)
        {
            let policy = package.policies.iter()
                .find(|policy| policy.export_id == membership.policy_export_id);
            let policy_destination = proposed_destination(
                membership.policy_export_id,
                &conflicts.policy_kinds,
                &conflicts.policy_local_ids,
            );
            if policy_destination == ProposedDestination::Unresolved {
                plan.unresolved_dependencies += 1;
            }
            members.push(ProposedPlaylistMember {
                position: membership.position,
                policy_package_id: membership.policy_export_id,
                policy_name: policy.map(|policy| policy.name.clone())
                    .unwrap_or_else(|| format!("Policy {}", membership.policy_export_id)),
                policy_destination,
            });
        }
        plan.playlists.push(ProposedPlaylistPlan {
            package_id: playlist.export_id,
            name: playlist.name.clone(),
            destination,
            members,
        });
    }

    plan
}

fn policy_values_match(policy: &PackagePolicy, local: &LocalPolicyValues) -> Result<bool, String> {
    let v = &policy.values;
    let get = |name: &str| v.get(name).map(String::as_str)
        .ok_or_else(|| format!("Validated policy '{}' lacks '{}'.", policy.name, name));
    let f64v = |name: &str| -> Result<f64,String> {
        let x=get(name)?; x.parse::<f64>().map_err(|_|format!("Invalid number '{}' in {}.",x,name))
    };
    let i64v = |name: &str| -> Result<i64,String> {
        let x=get(name)?; x.parse::<i64>().map_err(|_|format!("Invalid integer '{}' in {}.",x,name))
    };
    let boolv = |name: &str| -> Result<bool,String> {
        match get(name)? {"true"=>Ok(true),"false"=>Ok(false),x=>Err(format!("Invalid boolean '{}' in {}.",x,name))}
    };
    let close = |a:f64,b:f64| (a-b).abs() <= 0.000_001;

    let app = crate::manage_configuration::load_app_defaults()?;
    let target = match policy.target.as_str() {
        "screensaver" | "wallpaper" => Some(crate::manage_configuration::load_target_defaults(&policy.target)?),
        "unassigned" => None,
        other => return Err(format!("Unsupported imported policy target '{}'.", other)),
    };

    let (texture_mode, texture_family, texture_primitives) = match local.texture_mode.as_deref() {
        Some("specific") => ("specific", local.texture_family.clone().unwrap_or_default(), local.texture_primitives),
        Some("random") => ("random", String::new(), target.as_ref().map(|d| d.texture_primitives)),
        None => match target.as_ref() {
            Some(d) if d.texture_mode == "specific" => ("specific", d.texture_family.clone().unwrap_or_default(), Some(d.texture_primitives)),
            Some(d) if d.texture_mode == "random" => ("random", String::new(), Some(d.texture_primitives)),
            Some(d) => return Err(format!("Receiving {} defaults have unsupported texture mode '{}'.", d.target, d.texture_mode)),
            None => ("inherit_target", String::new(), None),
        },
        Some(other) => return Err(format!("Receiving policy has unsupported texture mode '{}'.", other)),
    };

    let (palette_mode, palette_color) = match local.palette_mode.as_deref() {
        Some("specific") => ("specific", local.palette_color.clone().unwrap_or_default()),
        Some("random") => ("random", String::new()),
        None => match target.as_ref() {
            Some(d) if d.palette_mode == "specific" => ("specific", d.palette_color.clone().unwrap_or_default()),
            Some(d) if d.palette_mode == "random" => ("random", String::new()),
            Some(d) => return Err(format!("Receiving {} defaults have unsupported palette mode '{}'.", d.target, d.palette_mode)),
            None => ("inherit_target", String::new()),
        },
        Some(other) => return Err(format!("Receiving policy has unsupported palette mode '{}'.", other)),
    };

    let (speed_mode, speed) = match local.animation_speed {
        Some(value) => ("explicit", Some(value)),
        None => match target.as_ref() {
            Some(d) => ("explicit", Some(d.animation_speed)),
            None => ("inherit_target", None),
        },
    };

    let imported_texture_primitives = if get("texture_primitives")?.is_empty() { None } else { Some(i64v("texture_primitives")?) };
    let imported_speed = if get("animation_speed")?.is_empty() { None } else { Some(f64v("animation_speed")?) };

    Ok(get("texture_mode")? == texture_mode
        && get("texture_family")? == texture_family
        && imported_texture_primitives == texture_primitives
        && get("palette_mode")? == palette_mode
        && get("palette_color")? == palette_color
        && i64v("rendered_fps")? == local.rendered_fps.unwrap_or(app.rendered_fps)
        && get("animation_speed_mode")? == speed_mode
        && match (imported_speed, speed) { (None,None)=>true,(Some(a),Some(b))=>close(a,b),_=>false }
        && close(local.starting_offset, f64v("starting_offset")?)
        && get("anti_aliasing")? == local.anti_aliasing.as_deref().unwrap_or(&app.anti_aliasing)
        && get("dithering")? == local.dithering.as_deref().unwrap_or(&app.dithering)
        && get("color_precision")? == local.color_precision.as_deref().unwrap_or(&app.color_precision)
        && close(local.render_scale.unwrap_or(app.render_scale), f64v("render_scale")?)
        && local.audiovisual_effect == get("audiovisual_effect")?
        && close(local.bloom_intensity, f64v("bloom_intensity")?)
        && close(local.bloom_saturation, f64v("bloom_saturation")?)
        && close(local.bloom_threshold, f64v("bloom_threshold")?)
        && close(local.bloom_frequency_rotation, f64v("bloom_frequency_rotation")?)
        && local.bloom_frequency_invert == boolv("bloom_frequency_invert")?
        && local.invert_colors == boolv("invert_colors")?
        && local.flip_horizontal == boolv("flip_horizontal")?
        && local.flip_vertical == boolv("flip_vertical")?
        && close(local.hue_rotation, f64v("hue_rotation")?))
}

fn draw_review(ui: &mut egui::Ui, state: &ImportWizardState) {
    ui.heading("Review & Confirm");
    ui.add_space(8.0);
    ui.label("Screenshaver is ready to apply the validated, dependency-resolved package to this installation.");
    ui.add_space(10.0);

    let Some(package) = state.package.as_ref() else {
        ui.label("No validated package is available.");
        return;
    };
    let Some(conflicts) = state.conflicts.as_ref() else {
        ui.label("Conflict discovery is unavailable.");
        return;
    };

    let (resolved_package, resolved_conflicts) = match resolved_package_and_report(package, conflicts, &state.resolutions) {
        Ok(value) => value,
        Err(_) => (package.clone(), conflicts.clone()),
    };
    let unresolved_dependencies = state.proposed_plan.as_ref()
        .map(|plan| plan.unresolved_dependencies)
        .unwrap_or(0);
    let conflict_count = unresolved_conflict_count(state);
    let conflicts = &resolved_conflicts;
    let package = &resolved_package;
    if conflict_count != 0 || unresolved_dependencies != 0 {
        ui.label(egui::RichText::new(format!(
            "Import is blocked: {} conflict(s), {} unresolved dependency reference(s).",
            conflict_count, unresolved_dependencies
        )).strong());
        ui.label(
            "Review is available for inspection, but Import will remain disabled if any deterministic rename or dependency destination cannot be generated."
        );
        ui.add_space(10.0);
    }

    let new_shaders = conflicts.shader_kinds.values().filter(|&&kind| kind == ConflictKind::New).count();
    let duplicate_shaders = conflicts.shader_kinds.values().filter(|&&kind| kind == ConflictKind::Duplicate).count();
    let new_policies = conflicts.policy_kinds.values().filter(|&&kind| kind == ConflictKind::New).count();
    let duplicate_policies = conflicts.policy_kinds.values().filter(|&&kind| kind == ConflictKind::Duplicate).count();
    let new_playlists = conflicts.playlist_kinds.values().filter(|&&kind| kind == ConflictKind::New).count();
    let duplicate_playlists = conflicts.playlist_kinds.values().filter(|&&kind| kind == ConflictKind::Duplicate).count();

    egui::Grid::new("screenshaver_import_review_summary")
        .num_columns(2)
        .spacing(egui::vec2(12.0, 4.0))
        .show(ui, |ui| {
            summary(ui, "Shaders:", format!("{} import, {} identical already present", new_shaders, duplicate_shaders));
            summary(ui, "Policies:", format!("{} import, {} identical already present", new_policies, duplicate_policies));
            summary(ui, "Playlists:", format!("{} import, {} identical already present", new_playlists, duplicate_playlists));
            summary(ui, "Playlist memberships:", package.memberships.len().to_string());
        });

    ui.add_space(12.0);
    ui.label(egui::RichText::new("Before Import").strong());
    ui.label("Screenshaver will create and verify a permanent timestamped backup of screenshaver.db before making persistent changes.");
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Import behavior").strong());
    ui.label("New and automatically renamed shader files will be installed in the managed shader folder, policies will be restored with their exported rendering settings, and playlists will be reconstructed in canonical order. Existing recipient objects are never modified. Truly identical duplicate objects are reused by imported dependencies.");
    ui.add_space(8.0);
    ui.label(egui::RichText::new("screenshaver.toml will not be changed.").strong());
}

fn draw_results(ui: &mut egui::Ui, state: &ImportWizardState) {
    ui.heading("Results");
    ui.add_space(8.0);
    let Some(result) = state.import_result.as_ref() else {
        ui.label("No Import result is available.");
        return;
    };
    ui.label(egui::RichText::new(if result.success { "Import completed successfully." } else { "Import failed." }).strong());
    ui.add_space(10.0);
    if result.success {
        egui::Grid::new("screenshaver_import_results_summary")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 4.0))
            .show(ui, |ui| {
                summary(ui, "Shaders created:", result.shaders_created.to_string());
                summary(ui, "Policies created:", result.policies_created.to_string());
                summary(ui, "Playlists created:", result.playlists_created.to_string());
                summary(ui, "Memberships created:", result.memberships_created.to_string());
            });
    }
    ui.add_space(10.0);
    ui.add(egui::Label::new(&result.detail).wrap());
    if let Some(path) = result.backup_path.as_ref() {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Pre-Import database backup:").strong());
        ui.add(egui::Label::new(path.display().to_string()).wrap());
    }
}

fn draw_navigation(ui: &mut egui::Ui, state: &mut ImportWizardState) {
    ui.horizontal(|ui| {
        if ui.add_enabled(
            matches!(state.stage, ImportStage::InspectArchive | ImportStage::ResolveConflicts | ImportStage::Review),
            egui::Button::new("< Back"),
        ).clicked() {
            state.stage = match state.stage {
                ImportStage::Review => ImportStage::ResolveConflicts,
                ImportStage::ResolveConflicts => ImportStage::InspectArchive,
                _ => ImportStage::SelectArchive,
            };
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if state.stage == ImportStage::Results { "Close" } else { "Cancel" }).clicked() {
                state.open = false;
            }

            ui.add_space(8.0);
            let next_enabled = match state.stage {
                ImportStage::SelectArchive => state.archive_selected(),
                ImportStage::InspectArchive => state.inspection_passed() && state.package.is_some(),
                // Review remains available even when conflicts exist so the user can
                // inspect the complete proposed operation. Persistent Import itself is
                // blocked until every conflict/dependency is resolved.
                ImportStage::ResolveConflicts => state.proposed_plan.as_ref().map(|p| p.unresolved_dependencies == 0).unwrap_or(false) && unresolved_conflict_count(state) == 0,
                // Review may invoke Import whenever a validated package, conflict report,
                // and proposed dependency plan are present. execute_clean_import() performs
                // the authoritative inspection/conflict/dependency checks again immediately
                // before any backup or persistent change, and refuses execution if anything
                // is unresolved. This avoids a stale UI-derived gate disabling an otherwise
                // valid clean Import while preserving the execution safety boundary.
                ImportStage::Review => state.package.is_some()
                    && state.conflicts.is_some()
                    && state.proposed_plan.as_ref().map(|p| p.unresolved_dependencies == 0).unwrap_or(false)
                    && unresolved_conflict_count(state) == 0,
                ImportStage::Results => false,
            };
            let button_text = if state.stage == ImportStage::Review { "Import" } else { "Next >" };

            if ui.add_enabled(next_enabled, egui::Button::new(button_text)).clicked() {
                match state.stage {
                    ImportStage::SelectArchive => {
                        let (inspection, package) = inspect_archive(Path::new(state.archive_path.trim()));
                        state.inspection = Some(inspection);
                        state.package = package;
                        state.stage = ImportStage::InspectArchive;
                    }
                    ImportStage::InspectArchive => {
                        if let Some(package) = state.package.as_ref() {
                            let conflicts = discover_conflicts(package);
                            state.conflict_rename_stamp = local_import_name_stamp().ok();
                            state.resolutions = state.conflict_rename_stamp.as_deref()
                                .map(|stamp| generate_automatic_conflict_resolutions(package, &conflicts, stamp))
                                .unwrap_or_default();
                            state.conflicts = Some(conflicts);
                            refresh_resolved_plan(state);
                            state.stage = ImportStage::ResolveConflicts;
                        }
                    }
                    ImportStage::ResolveConflicts => state.stage = ImportStage::Review,
                    ImportStage::Review => {
                        state.import_result = Some(execute_clean_import(Path::new(state.archive_path.trim()), &state.resolutions, state.conflict_rename_stamp.as_deref()));
                        state.stage = ImportStage::Results;
                    }
                    ImportStage::Results => {}
                }
            }
        });
    });
}

fn execute_clean_import(
    archive_path: &Path,
    resolutions: &BTreeMap<(String,u64), ConflictResolution>,
    conflict_rename_stamp: Option<&str>,
) -> ImportExecutionResult {
    // Re-inspect immediately before execution so an archive changed after the
    // Review page cannot be imported using stale validation results.
    let (inspection, package) = inspect_archive(archive_path);
    if !inspection.passed {
        return ImportExecutionResult::failure(
            "The archive no longer passes inspection. No Import changes were made.",
            None,
        );
    }
    let Some(package) = package else {
        return ImportExecutionResult::failure("Validated package is unavailable.", None);
    };

    let discovered = discover_conflicts(&package);
    let has_conflicts = discovered.items.iter().any(|item| item.kind == ConflictKind::Conflict);
    let rename_stamp = match (has_conflicts, conflict_rename_stamp) {
        (true, Some(stamp)) => stamp,
        (true, None) => return ImportExecutionResult::failure(
            "The Import conflict timestamp is unavailable. No Import changes were made.",
            None,
        ),
        (false, _) => "",
    };
    let refreshed_resolutions = generate_automatic_conflict_resolutions(&package, &discovered, rename_stamp);
    for (key, resolution) in resolutions {
        match refreshed_resolutions.get(key) {
            Some(refreshed) if refreshed.renamed_name == resolution.renamed_name => {}
            Some(refreshed) => return ImportExecutionResult::failure(
                format!("The receiving installation changed after Review: '{}' is no longer the deterministic destination name (now '{}'). No Import changes were made.", resolution.renamed_name, refreshed.renamed_name),
                None,
            ),
            None => return ImportExecutionResult::failure(
                "The receiving installation changed after Review and the conflict set is no longer the same. No Import changes were made.",
                None,
            ),
        }
    }
    if refreshed_resolutions.len() != resolutions.len() {
        return ImportExecutionResult::failure(
            "The receiving installation changed after Review and new conflicts were discovered. No Import changes were made.",
            None,
        );
    }
    let (package, conflicts) = match resolved_package_and_report(&package, &discovered, &refreshed_resolutions) {
        Ok(value) => value,
        Err(error) => return ImportExecutionResult::failure(format!("Import conflict resolution is incomplete or stale: {} No Import changes were made.", error), None),
    };
    let conflict_items = conflicts.items.iter()
        .filter(|item| item.kind == ConflictKind::Conflict)
        .map(|item| format!("{} '{}': {}", item.object_type, item.name, item.detail))
        .collect::<Vec<_>>();
    if !conflict_items.is_empty() {
        return ImportExecutionResult::failure(
            format!(
                "Execution-time conflict discovery found {} unresolved conflict(s):\n\n{}\n\nNo Import changes were made.",
                conflict_items.len(),
                conflict_items.join("\n")
            ),
            None,
        );
    }
    let plan = build_proposed_import_plan(&package, &conflicts);
    if plan.unresolved_dependencies != 0 {
        let mut unresolved = Vec::new();
        for policy in &plan.policies {
            if policy.destination == ProposedDestination::Unresolved {
                unresolved.push(format!("Policy '{}' (package ID {}) has an unresolved destination.", policy.name, policy.package_id));
            }
            if policy.shader_destination == ProposedDestination::Unresolved {
                unresolved.push(format!(
                    "Policy '{}' (package ID {}) requires unresolved shader package ID {}.",
                    policy.name, policy.package_id, policy.shader_package_id
                ));
            }
        }
        for playlist in &plan.playlists {
            if playlist.destination == ProposedDestination::Unresolved {
                unresolved.push(format!("Playlist '{}' (package ID {}) has an unresolved destination.", playlist.name, playlist.package_id));
            }
            for member in &playlist.members {
                if member.policy_destination == ProposedDestination::Unresolved {
                    unresolved.push(format!(
                        "Playlist '{}' member {} '{}' requires unresolved policy package ID {}.",
                        playlist.name, member.position, member.policy_name, member.policy_package_id
                    ));
                }
            }
        }
        return ImportExecutionResult::failure(
            format!(
                "Execution-time dependency validation found {} unresolved dependency reference(s):\n\n{}\n\nNo Import changes were made.",
                plan.unresolved_dependencies,
                unresolved.join("\n")
            ),
            None,
        );
    }

    let database_path = crate::locate_paths::database_path();
    let backup_path = match create_verified_database_backup(&database_path) {
        Ok(path) => path,
        Err(error) => return ImportExecutionResult::failure(error, None),
    };

    match execute_clean_import_after_backup(archive_path, &package, &conflicts) {
        Ok(mut result) => {
            result.backup_path = Some(backup_path);
            result
        }
        Err(error) => {
            let restore = restore_database_backup(&database_path, &backup_path);
            let detail = match restore {
                Ok(()) => format!("{} The pre-Import database was restored. Newly installed shader files were removed where possible.", error),
                Err(restore_error) => format!("{} DATABASE RESTORE ALSO FAILED: {}. The verified backup remains at '{}'.", error, restore_error, backup_path.display()),
            };
            ImportExecutionResult::failure(detail, Some(backup_path))
        }
    }
}

fn create_verified_database_backup(database_path: &Path) -> Result<PathBuf, String> {
    let connection = crate::open_database::open()?;
    let stamp: String = connection.query_row(
        "SELECT strftime('%Y%m%d-%H%M%S', 'now', 'localtime')",
        [],
        |row| row.get(0),
    ).map_err(|error| format!("Unable to determine pre-Import backup timestamp: {}", error))?;

    let base_name = database_path.file_name().and_then(|value| value.to_str()).unwrap_or("screenshaver.db");
    let parent = database_path.parent().unwrap_or_else(|| Path::new("."));
    let mut ordinal = 1_u32;
    let backup_path = loop {
        let suffix = if ordinal == 1 { String::new() } else { format!("({})", ordinal) };
        let candidate = parent.join(format!("{}.{}{}", base_name, stamp, suffix));
        if !candidate.exists() { break candidate; }
        ordinal += 1;
    };

    // VACUUM INTO is SQLite's consistent snapshot operation. The destination is
    // generated locally rather than from archive data; quote it defensively.
    let quoted = backup_path.to_string_lossy().replace('\'', "''");
    connection.execute_batch(&format!("VACUUM INTO '{}';", quoted))
        .map_err(|error| format!("Unable to create pre-Import database backup '{}': {}", backup_path.display(), error))?;
    drop(connection);

    let verify = rusqlite::Connection::open_with_flags(
        &backup_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ).map_err(|error| format!("Unable to open pre-Import database backup for verification: {}", error))?;
    let integrity: String = verify.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("Unable to verify pre-Import database backup: {}", error))?;
    if integrity != "ok" {
        return Err(format!("Pre-Import database backup failed integrity verification: {}", integrity));
    }
    Ok(backup_path)
}

fn restore_database_backup(database_path: &Path, backup_path: &Path) -> Result<(), String> {
    std::fs::copy(backup_path, database_path)
        .map_err(|error| format!("Unable to restore '{}' from '{}': {}", database_path.display(), backup_path.display(), error))?;
    let verify = rusqlite::Connection::open_with_flags(database_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Unable to open restored database: {}", error))?;
    let integrity: String = verify.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("Unable to verify restored database: {}", error))?;
    if integrity == "ok" { Ok(()) } else { Err(format!("Restored database integrity_check returned '{}'.", integrity)) }
}

fn execute_clean_import_after_backup(
    archive_path: &Path,
    package: &ValidatedPackage,
    conflicts: &ConflictReport,
) -> Result<ImportExecutionResult, String> {
    let shader_dir = crate::locate_paths::shader_dir();
    std::fs::create_dir_all(&shader_dir)
        .map_err(|error| format!("Unable to create managed shader directory '{}': {}", shader_dir.display(), error))?;

    let mut created_files = Vec::<PathBuf>::new();
    let operation = (|| -> Result<ImportExecutionResult, String> {
        let file = std::fs::File::open(archive_path)
            .map_err(|error| format!("Unable to reopen Import archive: {}", error))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|error| format!("Unable to reopen Import ZIP: {}", error))?;

        for shader in package.shaders.iter().filter(|shader| conflicts.shader_kinds.get(&shader.export_id) == Some(&ConflictKind::New)) {
            let destination = shader_dir.join(&shader.filename);
            if destination.exists() {
                return Err(format!("Refusing to overwrite unexpected existing shader file '{}'.", destination.display()));
            }
            let mut entry = zip.by_name(&shader.archive_path)
                .map_err(|error| format!("Unable to read shader payload '{}': {}", shader.archive_path, error))?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)
                .map_err(|error| format!("Unable to read shader payload '{}': {}", shader.archive_path, error))?;
            if sha256_hex(&bytes) != shader.sha256 {
                return Err(format!("Shader '{}' changed after inspection; SHA-256 no longer matches.", shader.filename));
            }
            std::fs::write(&destination, &bytes)
                .map_err(|error| format!("Unable to install shader '{}': {}", destination.display(), error))?;
            created_files.push(destination);
        }
        drop(zip);

        let mut connection = crate::open_database::open()?;
        crate::reconcile_shaders::reconcile(&mut connection)
            .map_err(|error| format!("Unable to reconcile imported shader files: {}", error))?;

        let managed_source = shader_dir.to_string_lossy().to_string();
        let mut shader_map = conflicts.shader_local_ids.clone();
        for shader in package.shaders.iter().filter(|shader| conflicts.shader_kinds.get(&shader.export_id) == Some(&ConflictKind::New)) {
            let local_id: i64 = connection.query_row(
                "SELECT shader_id FROM shaders WHERE source_path = ?1 AND filename = ?2 AND source_hash = ?3 ORDER BY shader_id LIMIT 1",
                rusqlite::params![&managed_source, &shader.filename, &shader.sha256],
                |row| row.get(0),
            ).map_err(|error| format!("Imported shader '{}' was not registered as expected: {}", shader.filename, error))?;
            shader_map.insert(shader.export_id, local_id);
        }

        let transaction = connection.transaction()
            .map_err(|error| format!("Unable to begin Import database transaction: {}", error))?;
        let mut policy_map = conflicts.policy_local_ids.clone();
        let mut policies_created = 0_usize;

        for policy in package.policies.iter().filter(|policy| conflicts.policy_kinds.get(&policy.export_id) == Some(&ConflictKind::New)) {
            let shader_id = shader_map.get(&policy.shader_export_id).copied()
                .ok_or_else(|| format!("Policy '{}' has no resolved destination shader.", policy.name))?;
            let policy_id = insert_imported_policy(&transaction, policy, shader_id)?;
            policy_map.insert(policy.export_id, policy_id);
            policies_created += 1;
        }

        let mut playlist_map = conflicts.playlist_local_ids.clone();
        let mut playlists_created = 0_usize;
        for playlist in package.playlists.iter().filter(|playlist| conflicts.playlist_kinds.get(&playlist.export_id) == Some(&ConflictKind::New)) {
            let name = playlist.name.trim();
            let key = import_name_key(name)?;
            let description = if playlist.description.trim().is_empty() { None } else { Some(playlist.description.trim()) };
            transaction.execute(
                "INSERT INTO playlists (playlist_created_at, playlist_modified_at, playlist_name, playlist_name_key, description) VALUES (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?1, ?2, ?3)",
                rusqlite::params![name, key, description],
            ).map_err(|error| format!("Unable to import playlist '{}': {}", playlist.name, error))?;
            playlist_map.insert(playlist.export_id, transaction.last_insert_rowid());
            playlists_created += 1;
        }

        let mut memberships_created = 0_usize;
        for membership in &package.memberships {
            let playlist_kind = conflicts.playlist_kinds.get(&membership.playlist_export_id).copied();
            if playlist_kind != Some(ConflictKind::New) {
                // A duplicate playlist already contains the identical canonical
                // membership set and therefore needs no database mutation.
                continue;
            }
            let playlist_id = playlist_map.get(&membership.playlist_export_id).copied()
                .ok_or_else(|| format!("Playlist package ID {} has no destination mapping.", membership.playlist_export_id))?;
            let policy_id = policy_map.get(&membership.policy_export_id).copied()
                .ok_or_else(|| format!("Policy package ID {} has no destination mapping for playlist membership.", membership.policy_export_id))?;
            transaction.execute(
                "INSERT INTO playlist_members (playlist_id, policy_id, position) VALUES (?1, ?2, ?3)",
                rusqlite::params![playlist_id, policy_id, membership.position as i64],
            ).map_err(|error| format!("Unable to restore playlist membership at position {}: {}", membership.position, error))?;
            memberships_created += 1;
        }

        transaction.commit().map_err(|error| format!("Unable to commit Import database transaction: {}", error))?;

        Ok(ImportExecutionResult {
            success: true,
            detail: "The validated package was imported successfully. Genuine conflicts were preserved additively under deterministic imported names; existing recipient objects were not modified. Truly identical duplicates were reused, and imported dependencies were mapped to their destination identities. screenshaver.toml was not changed.".to_string(),
            backup_path: None,
            shaders_created: created_files.len(),
            policies_created,
            playlists_created,
            memberships_created,
        })
    })();

    if operation.is_err() {
        for path in created_files.iter().rev() {
            let _ = std::fs::remove_file(path);
        }
    }
    operation
}

fn insert_imported_policy(
    transaction: &rusqlite::Transaction<'_>,
    policy: &PackagePolicy,
    shader_id: i64,
) -> Result<i64, String> {
    let v = &policy.values;
    let get = |name: &str| v.get(name).map(String::as_str)
        .ok_or_else(|| format!("Imported policy '{}' lacks '{}'.", policy.name, name));
    let parse_i64 = |name: &str| -> Result<i64, String> {
        let value = get(name)?;
        value.parse::<i64>().map_err(|_| format!("Invalid integer '{}' in imported policy field '{}'.", value, name))
    };
    let parse_f64 = |name: &str| -> Result<f64, String> {
        let value = get(name)?;
        value.parse::<f64>().map_err(|_| format!("Invalid number '{}' in imported policy field '{}'.", value, name))
    };
    let parse_bool = |name: &str| -> Result<i64, String> {
        match get(name)? { "true" => Ok(1), "false" => Ok(0), value => Err(format!("Invalid boolean '{}' in imported policy field '{}'.", value, name)) }
    };

    let texture_mode = get("texture_mode")?;
    let (db_texture_mode, texture_family, texture_primitives): (Option<&str>, Option<&str>, Option<i64>) = match texture_mode {
        "specific" => (Some("specific"), Some(get("texture_family")?), Some(parse_i64("texture_primitives")?)),
        "random" => (Some("random"), None, None),
        "inherit_target" => (None, None, None),
        other => return Err(format!("Unsupported imported texture mode '{}'.", other)),
    };
    let palette_mode = get("palette_mode")?;
    let (db_palette_mode, palette_color): (Option<&str>, Option<&str>) = match palette_mode {
        "specific" => (Some("specific"), Some(get("palette_color")?)),
        "random" => (Some("random"), None),
        "inherit_target" => (None, None),
        other => return Err(format!("Unsupported imported palette mode '{}'.", other)),
    };
    let animation_speed = match get("animation_speed_mode")? {
        "explicit" => Some(parse_f64("animation_speed")?),
        "inherit_target" => None,
        other => return Err(format!("Unsupported imported animation speed mode '{}'.", other)),
    };

    let name = policy.name.trim();
    let key = import_name_key(name)?;
    transaction.execute(
        "INSERT INTO shader_policies (policy_created_at, policy_modified_at, policy_name, policy_name_key, shader_id, policy_target, texture_mode, texture_family, texture_primitives, palette_mode, palette_color, rendered_fps, animation_speed, starting_offset, anti_aliasing, dithering, color_precision, render_scale, audiovisual_effect, bloom_intensity, bloom_saturation, bloom_threshold, bloom_frequency_rotation, bloom_frequency_invert, invert_colors, flip_horizontal, flip_vertical, hue_rotation) VALUES (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
        rusqlite::params![
            name, key, shader_id, &policy.target, db_texture_mode, texture_family, texture_primitives,
            db_palette_mode, palette_color, parse_i64("rendered_fps")?, animation_speed,
            parse_f64("starting_offset")?, get("anti_aliasing")?, get("dithering")?, get("color_precision")?,
            parse_f64("render_scale")?, get("audiovisual_effect")?, parse_f64("bloom_intensity")?,
            parse_f64("bloom_saturation")?, parse_f64("bloom_threshold")?, parse_f64("bloom_frequency_rotation")?,
            parse_bool("bloom_frequency_invert")?, parse_bool("invert_colors")?, parse_bool("flip_horizontal")?,
            parse_bool("flip_vertical")?, parse_f64("hue_rotation")?,
        ],
    ).map_err(|error| format!("Unable to import policy '{}': {}", policy.name, error))?;
    Ok(transaction.last_insert_rowid())
}

fn import_name_key(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    let length = trimmed.chars().count();
    if !(1..=128).contains(&length) {
        return Err(format!("Imported name must contain between 1 and 128 characters; found {}.", length));
    }
    let key = trimmed.chars().flat_map(|character| character.to_lowercase()).collect::<String>();
    if key.is_empty() { Err("Imported name produced an empty comparison key.".into()) } else { Ok(key) }
}

fn starting_directory(archive_path: &str) -> PathBuf {
    let path = Path::new(archive_path.trim());
    if let Some(parent) = path.parent() {
        if parent.is_dir() {
            return parent.to_path_buf();
        }
    }

    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn schema_for_version(version: u32) -> Result<ExportSchema, String> {
    let text = match version {
        1 => include_str!("../assets/export/schema_v001.json"),
        _ => return Err(format!(
            "Screenshaver Export Format {} is not supported by this installation.",
            version
        )),
    };

    let schema: ExportSchema = serde_json::from_str(text)
        .map_err(|error| format!("Unable to load Export Schema {}: {}", version, error))?;

    if schema.format_version != version {
        return Err("Export Schema version does not match the archive.".into());
    }

    if schema.integrity.algorithm != "sha256"
        || schema.integrity.package_canonicalization != "screenshaver-package-v1"
    {
        return Err("Export Schema requests unsupported integrity behavior.".into());
    }

    if !schema.integrity.package_hash_excludes.iter()
        .any(|name| name == &schema.archive.manifest)
    {
        return Err("Export Schema must exclude its manifest from package hashing.".into());
    }

    for file in [
        &schema.datasets.policies.file,
        &schema.datasets.shaders.file,
        &schema.datasets.playlists.file,
        &schema.datasets.playlist_members.file,
    ] {
        if !schema.archive.metadata_files.iter().any(|name| name == file) {
            return Err(format!("Schema metadata file '{}' is not declared in the archive.", file));
        }
    }

    Ok(schema)
}

fn inspect_archive(path: &Path) -> (ArchiveInspection, Option<ValidatedPackage>) {
    let mut result = ArchiveInspection::new();

    let metadata = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            result.fail("Archive file", "Selected path is not a regular file.");
            result.finish();
            return (result, None);
        }
        Err(error) => {
            result.fail("Archive file", format!("Unable to access archive: {}", error));
            result.finish();
            return (result, None);
        }
    };

    if metadata.len() > MAX_ZIP_BYTES {
        result.fail("Archive size", format!(
            "{} bytes exceeds the {} byte inspection limit.",
            metadata.len(), MAX_ZIP_BYTES
        ));
        result.finish();
        return (result, None);
    }
    result.pass("Archive file", format!("{} bytes", metadata.len()));

    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            result.fail("ZIP container", format!("Unable to open archive: {}", error));
            result.finish();
            return (result, None);
        }
    };

    let mut zip = match zip::ZipArchive::new(file) {
        Ok(zip) => zip,
        Err(error) => {
            result.fail("ZIP container", format!("Invalid ZIP archive: {}", error));
            result.finish();
            return (result, None);
        }
    };

    if zip.len() > MAX_ENTRIES {
        result.fail("ZIP entry count", format!(
            "{} entries exceeds the {} entry limit.", zip.len(), MAX_ENTRIES
        ));
        result.finish();
        return (result, None);
    }

    let mut entries = BTreeMap::<String, Payload>::new();
    let mut duplicate = BTreeSet::new();
    let mut unsafe_names = Vec::new();
    let mut special = Vec::new();
    let mut expanded = 0_u64;

    for index in 0..zip.len() {
        let mut entry = match zip.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                result.fail("ZIP entry access", format!("Entry {}: {}", index, error));
                result.finish();
                return (result, None);
            }
        };

        let name = entry.name().to_string();
        if !safe_name(&name) {
            unsafe_names.push(name.clone());
        }
        if entries.contains_key(&name) {
            duplicate.insert(name.clone());
        }

        if let Some(mode) = entry.unix_mode() {
            let kind = mode & 0o170000;
            if kind != 0 && kind != 0o100000 && kind != 0o040000 {
                special.push(name.clone());
            }
        }

        expanded = expanded.saturating_add(entry.size());
        if expanded > MAX_EXPANDED_BYTES || entry.size() > MAX_ENTRY_BYTES {
            result.fail("Archive resource limits",
                "Archive expansion or individual entry exceeds inspection limits.");
            result.finish();
            return (result, None);
        }

        let is_dir = entry.is_dir();
        let unix_mode = entry.unix_mode();
        let mut bytes = Vec::new();
        if !is_dir {
            if let Err(error) = entry.read_to_end(&mut bytes) {
                result.fail("Archive member read", format!("{}: {}", name, error));
                result.finish();
                return (result, None);
            }
        }

        entries.insert(name, Payload { bytes, is_dir, unix_mode });
    }

    result.pass("ZIP container", format!("{} entries; {} expanded bytes", zip.len(), expanded));

    if duplicate.is_empty() {
        result.pass("Unique archive names", "No duplicate ZIP member names detected.");
    } else {
        result.fail("Unique archive names", duplicate.into_iter().collect::<Vec<_>>().join(", "));
    }

    if unsafe_names.is_empty() {
        result.pass("Archive paths", "No absolute, traversal, NUL, or backslash paths detected.");
    } else {
        result.fail("Archive paths", unsafe_names.join(", "));
    }

    if special.is_empty() {
        result.pass("Archive entry types", "No symlink or special-file entries detected.");
    } else {
        result.fail("Archive entry types", special.join(", "));
    }

    let manifest_payload = match entries.get("manifest.json") {
        Some(payload) if !payload.is_dir => payload,
        _ => {
            result.fail("Manifest", "Required manifest.json is missing.");
            result.finish();
            return (result, None);
        }
    };

    let manifest: Manifest = match serde_json::from_slice(&manifest_payload.bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            result.fail("Manifest", format!("manifest.json is invalid: {}", error));
            result.finish();
            return (result, None);
        }
    };

    result.format_version = Some(manifest.format_version);
    result.source_version = Some(manifest.screenshaver_version.clone());
    result.source_db_schema = Some(manifest.database_schema_version);
    result.export_focus = Some(manifest.export_focus.clone());

    let schema = match schema_for_version(manifest.format_version) {
        Ok(schema) => {
            result.pass("Export schema",
                format!("Screenshaver Export Format {} is supported.", manifest.format_version));
            schema
        }
        Err(error) => {
            result.fail("Export schema", error);
            result.finish();
            return (result, None);
        }
    };

    if manifest.format == schema.format {
        result.pass("Format identifier", manifest.format.clone());
    } else {
        result.fail("Format identifier", format!(
            "Manifest '{}'; schema '{}'.", manifest.format, schema.format
        ));
    }

    let shader_table = match parse_tsv(
        &entries.get(&schema.datasets.shaders.file)
            .map(|p| p.bytes.as_slice()).unwrap_or(&[]),
        &schema.datasets.shaders,
    ) {
        Ok(table) => table,
        Err(error) => {
            result.fail("Shader metadata structure", error);
            result.finish();
            return (result, None);
        }
    };

    let archive_path_col = match column(&schema.datasets.shaders, "archive_path") {
        Ok(index) => index,
        Err(error) => {
            result.fail("Export schema", error);
            result.finish();
            return (result, None);
        }
    };

    let mut expected = BTreeSet::new();
    expected.insert(schema.archive.manifest.clone());
    expected.extend(schema.archive.metadata_files.iter().cloned());

    let shader_prefix = format!("{}/", schema.archive.shader_directory.trim_end_matches('/'));
    for row in &shader_table.rows {
        let archive_path = &row[archive_path_col];
        if !safe_name(archive_path) || !archive_path.starts_with(&shader_prefix) {
            result.fail("Shader archive paths",
                format!("'{}' is not permitted by Export Schema.", archive_path));
        }
        expected.insert(archive_path.clone());
    }

    let actual = entries.iter()
        .filter(|(_, payload)| !payload.is_dir)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();

    let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
    let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();

    if missing.is_empty() && unexpected.is_empty() {
        result.pass("Package structure",
            "All required members are present and no unexpected files were found.");
    } else {
        if !missing.is_empty() {
            result.fail("Missing archive content", missing.join(", "));
        }
        if !unexpected.is_empty() {
            result.fail("Unexpected archive content", unexpected.join(", "));
        }
    }

    let policy_table = inspect_metadata_file(
        "Policies", &schema.datasets.policies, &manifest, &entries, &mut result);
    let shader_table = inspect_metadata_file(
        "Shaders", &schema.datasets.shaders, &manifest, &entries, &mut result);
    let playlist_table = inspect_metadata_file(
        "Playlists", &schema.datasets.playlists, &manifest, &entries, &mut result);
    let membership_table = inspect_metadata_file(
        "Playlist memberships", &schema.datasets.playlist_members, &manifest, &entries, &mut result);

    let (Ok(policies), Ok(shaders), Ok(playlists), Ok(memberships)) =
        (policy_table, shader_table, playlist_table, membership_table)
    else {
        result.finish();
        return (result, None);
    };

    result.policies = policies.rows.len();
    result.shaders = shaders.rows.len();
    result.playlists = playlists.rows.len();
    result.memberships = memberships.rows.len();

    if result.policies == manifest.policy_count
        && result.shaders == manifest.shader_count
        && result.playlists == manifest.playlist_count
    {
        result.pass("Manifest counts", "Policy, shader, and playlist counts match metadata.");
    } else {
        result.fail("Manifest counts", "Manifest counts do not match parsed metadata.");
    }

    inspect_relationships(
        &schema, &policies, &shaders, &playlists, &memberships, &mut result);
    inspect_shaders(&schema, &shaders, &entries, &mut result);
    inspect_package_hash(&schema, &manifest, &entries, &mut result);

    let package = if result.checks.iter().all(|check| check.passed) {
        match build_validated_package(
            &schema,
            &policies,
            &shaders,
            &playlists,
            &memberships,
        ) {
            Ok(package) => Some(package),
            Err(error) => {
                result.fail("Validated package", error);
                None
            }
        }
    } else {
        None
    };

    result.finish();
    (result, package)
}


fn build_validated_package(
    schema: &ExportSchema,
    policies: &Tsv,
    shaders: &Tsv,
    playlists: &Tsv,
    memberships: &Tsv,
) -> Result<ValidatedPackage, String> {
    let policy_id = column(&schema.datasets.policies, "policy_export_id")?;
    let policy_name = column(&schema.datasets.policies, "policy_name")?;
    let policy_target = column(&schema.datasets.policies, "policy_target")?;
    let policy_shader = column(&schema.datasets.policies, "shader_export_id")?;

    let shader_id = column(&schema.datasets.shaders, "shader_export_id")?;
    let shader_filename = column(&schema.datasets.shaders, "filename")?;
    let shader_archive_path = column(&schema.datasets.shaders, "archive_path")?;
    let shader_hash = column(&schema.datasets.shaders, "sha256")?;

    let playlist_id = column(&schema.datasets.playlists, "playlist_export_id")?;
    let playlist_name = column(&schema.datasets.playlists, "playlist_name")?;
    let playlist_description = column(&schema.datasets.playlists, "description")?;

    let member_playlist = column(&schema.datasets.playlist_members, "playlist_export_id")?;
    let member_policy = column(&schema.datasets.playlist_members, "policy_export_id")?;
    let member_position = column(&schema.datasets.playlist_members, "position")?;

    let policies = policies.rows.iter().map(|row| {
        Ok(PackagePolicy {
            export_id: positive_id(&row[policy_id], "policy_export_id")?,
            name: row[policy_name].clone(),
            target: row[policy_target].clone(),
            shader_export_id: positive_id(&row[policy_shader], "shader_export_id")?,
            values: schema.datasets.policies.columns.iter().cloned()
                .zip(row.iter().cloned()).collect(),
        })
    }).collect::<Result<Vec<_>, String>>()?;

    let shaders = shaders.rows.iter().map(|row| {
        Ok(PackageShader {
            export_id: positive_id(&row[shader_id], "shader_export_id")?,
            filename: row[shader_filename].clone(),
            archive_path: row[shader_archive_path].clone(),
            sha256: row[shader_hash].clone(),
        })
    }).collect::<Result<Vec<_>, String>>()?;

    let playlists = playlists.rows.iter().map(|row| {
        Ok(PackagePlaylist {
            export_id: positive_id(&row[playlist_id], "playlist_export_id")?,
            name: row[playlist_name].clone(),
            description: row[playlist_description].clone(),
        })
    }).collect::<Result<Vec<_>, String>>()?;

    let memberships = memberships.rows.iter().map(|row| {
        Ok(PackageMembership {
            playlist_export_id: positive_id(&row[member_playlist], "playlist_export_id")?,
            policy_export_id: positive_id(&row[member_policy], "policy_export_id")?,
            position: positive_id(&row[member_position], "position")?,
        })
    }).collect::<Result<Vec<_>, String>>()?;

    // Preserve canonical sender ordering for later conflict-resolution mapping.
    let mut memberships = memberships;
    memberships.sort_by_key(|member| (member.playlist_export_id, member.position));

    Ok(ValidatedPackage {
        policies,
        shaders,
        playlists,
        memberships,
    })
}

fn inspect_metadata_file(
    label: &str,
    dataset: &SchemaDataset,
    manifest: &Manifest,
    entries: &BTreeMap<String, Payload>,
    result: &mut ArchiveInspection,
) -> Result<Tsv, ()> {
    let Some(payload) = entries.get(&dataset.file) else {
        result.fail(format!("{} metadata", label),
            format!("Required '{}' is missing.", dataset.file));
        return Err(());
    };

    let Some(expected) = manifest.files.get(&dataset.file) else {
        result.fail(format!("{} metadata hash", label),
            "Manifest integrity record is missing.");
        return Err(());
    };

    if valid_hash(&expected.sha256) && sha256_hex(&payload.bytes) == expected.sha256 {
        result.pass(format!("{} metadata hash", label), "SHA-256 verified.");
    } else {
        result.fail(format!("{} metadata hash", label), "SHA-256 mismatch or malformed hash.");
    }

    match parse_tsv(&payload.bytes, dataset) {
        Ok(table) => {
            result.pass(format!("{} structure", label),
                format!("{} rows; schema header verified.", table.rows.len()));
            Ok(table)
        }
        Err(error) => {
            result.fail(format!("{} structure", label), error);
            Err(())
        }
    }
}

fn parse_tsv(bytes: &[u8], dataset: &SchemaDataset) -> Result<Tsv, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("'{}' is not UTF-8: {}", dataset.file, error))?;

    let mut lines = text.lines();
    let header = lines.next()
        .ok_or_else(|| format!("'{}' is empty.", dataset.file))?
        .split('\t').collect::<Vec<_>>();

    if header != dataset.columns.iter().map(String::as_str).collect::<Vec<_>>() {
        return Err(format!("'{}' header does not match Export Schema.", dataset.file));
    }

    let mut rows = Vec::new();
    for (number, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = line.split('\t')
            .map(unescape)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("'{}' row {}: {}", dataset.file, number + 2, error))?;

        if row.len() != dataset.columns.len() {
            return Err(format!(
                "'{}' row {} has {} columns; {} required.",
                dataset.file, number + 2, row.len(), dataset.columns.len()
            ));
        }
        rows.push(row);
    }

    Ok(Tsv { rows })
}

fn unescape(value: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let escaped = chars.next().ok_or("Trailing TSV escape.")?;
        match escaped {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'n' => out.push('\n'),
            other => return Err(format!("Unsupported TSV escape '\\{}'.", other)),
        }
    }
    Ok(out)
}

fn column(dataset: &SchemaDataset, name: &str) -> Result<usize, String> {
    dataset.columns.iter().position(|column| column == name)
        .ok_or_else(|| format!("Schema dataset '{}' lacks column '{}'.", dataset.file, name))
}

fn ids(table: &Tsv, index: usize, label: &str) -> Result<HashSet<u64>, String> {
    let mut result = HashSet::new();
    for row in &table.rows {
        let id = positive_id(&row[index], label)?;
        if !result.insert(id) {
            return Err(format!("Duplicate {} {}.", label, id));
        }
    }
    Ok(result)
}

fn positive_id(value: &str, label: &str) -> Result<u64, String> {
    let id = value.parse::<u64>()
        .map_err(|_| format!("{} '{}' is not a positive integer.", label, value))?;
    if id == 0 {
        Err(format!("{} must be greater than zero.", label))
    } else {
        Ok(id)
    }
}

fn inspect_relationships(
    schema: &ExportSchema,
    policies: &Tsv,
    shaders: &Tsv,
    playlists: &Tsv,
    memberships: &Tsv,
    result: &mut ArchiveInspection,
) {
    let outcome = (|| -> Result<(), String> {
        let shader_id = column(&schema.datasets.shaders, "shader_export_id")?;
        let policy_id = column(&schema.datasets.policies, "policy_export_id")?;
        let policy_shader = column(&schema.datasets.policies, "shader_export_id")?;
        let playlist_id = column(&schema.datasets.playlists, "playlist_export_id")?;
        let member_playlist = column(&schema.datasets.playlist_members, "playlist_export_id")?;
        let member_policy = column(&schema.datasets.playlist_members, "policy_export_id")?;
        let member_position = column(&schema.datasets.playlist_members, "position")?;

        let shader_ids = ids(shaders, shader_id, "shader_export_id")?;
        let policy_ids = ids(policies, policy_id, "policy_export_id")?;
        let playlist_ids = ids(playlists, playlist_id, "playlist_export_id")?;

        for row in &policies.rows {
            let id = positive_id(&row[policy_shader], "policy shader_export_id")?;
            if !shader_ids.contains(&id) {
                return Err(format!("Policy references missing shader_export_id {}.", id));
            }
        }

        let mut pairs = HashSet::new();
        let mut positions = HashSet::new();
        for row in &memberships.rows {
            let pl = positive_id(&row[member_playlist], "membership playlist_export_id")?;
            let po = positive_id(&row[member_policy], "membership policy_export_id")?;
            let pos = positive_id(&row[member_position], "membership position")?;

            if !playlist_ids.contains(&pl) || !policy_ids.contains(&po) {
                return Err("Playlist membership contains an unresolved package ID.".into());
            }
            if !pairs.insert((pl, po)) {
                return Err(format!("Playlist {} contains policy {} more than once.", pl, po));
            }
            if !positions.insert((pl, pos)) {
                return Err(format!("Playlist {} contains duplicate position {}.", pl, pos));
            }
        }
        Ok(())
    })();

    match outcome {
        Ok(()) => result.pass("Package relationships",
            "Policy→shader and playlist→policy references are structurally valid."),
        Err(error) => result.fail("Package relationships", error),
    }
}

fn inspect_shaders(
    schema: &ExportSchema,
    shaders: &Tsv,
    entries: &BTreeMap<String, Payload>,
    result: &mut ArchiveInspection,
) {
    let outcome = (|| -> Result<(), String> {
        let path_col = column(&schema.datasets.shaders, "archive_path")?;
        let hash_col = column(&schema.datasets.shaders, "sha256")?;
        let mut paths = HashSet::new();

        for row in &shaders.rows {
            let path = &row[path_col];
            let expected = &row[hash_col];

            if !paths.insert(path.clone()) {
                return Err(format!("Shader '{}' is declared more than once.", path));
            }
            if !valid_hash(expected) {
                return Err(format!("Shader '{}' has malformed SHA-256.", path));
            }

            let payload = entries.get(path)
                .ok_or_else(|| format!("Declared shader '{}' is missing.", path))?;

            if payload.is_dir {
                return Err(format!("Declared shader '{}' is a directory.", path));
            }
            if payload.unix_mode.map(|mode| mode & 0o111 != 0).unwrap_or(false) {
                return Err(format!("Shader '{}' is marked executable.", path));
            }
            if sha256_hex(&payload.bytes) != *expected {
                return Err(format!("Shader '{}' failed SHA-256 verification.", path));
            }
            std::str::from_utf8(&payload.bytes)
                .map_err(|_| format!("Shader '{}' is not valid UTF-8 text.", path))?;
        }
        Ok(())
    })();

    match outcome {
        Ok(()) => {
            result.pass("Shader payloads",
                format!("{} declared shader files are present.", shaders.rows.len()));
            result.pass("Shader integrity", "Every shader SHA-256 value verified.");
            result.pass("Shader source encoding", "Every shader payload is valid UTF-8 text.");
        }
        Err(error) => result.fail("Shader payload inspection", error),
    }
}

fn inspect_package_hash(
    schema: &ExportSchema,
    manifest: &Manifest,
    entries: &BTreeMap<String, Payload>,
    result: &mut ArchiveInspection,
) {
    if !valid_hash(&manifest.package_sha256) {
        result.fail("Package SHA-256", "Manifest package hash is malformed.");
        return;
    }

    let excluded = schema.integrity.package_hash_excludes.iter()
        .cloned().collect::<HashSet<_>>();

    let mut records = entries.iter()
        .filter(|(name, payload)| !payload.is_dir && !excluded.contains(*name))
        .map(|(name, payload)| (name.clone(), payload.bytes.as_slice()))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.0.cmp(&right.0));

    let mut canonical = Vec::new();
    for (name, bytes) in records {
        canonical.extend_from_slice(name.as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(bytes.len().to_string().as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(sha256_hex(bytes).as_bytes());
        canonical.push(b'\n');
    }

    if sha256_hex(&canonical) == manifest.package_sha256 {
        result.pass("Package SHA-256", "Canonical package fingerprint verified.");
    } else {
        result.fail("Package SHA-256", "Canonical package fingerprint does not match manifest.");
    }
}

fn safe_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\0') || name.contains('\\') || name.starts_with('/') {
        return false;
    }
    Path::new(name).components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
