// Export Data wizard UI.
//
// This module owns the transient Control Center workflow for exporting portable
// Screenshaver data. The user first chooses an Export Focus: Policies, Shaders,
// or Playlists. That focus is the selectable root of the export; the other
// categories are derived read-only. Selected/derived policies are then resolved
// into an in-memory portable effective configuration and writes Screenshaver
// Export Format 1 as a ZIP archive after final confirmation.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExportStage {
    SelectFocus,
    SelectData,
    Destination,
    Review,
    Results,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExportSelectionRoot {
    Policies,
    Shaders,
    Playlists,
}

impl ExportSelectionRoot {
    const ALL: [ExportSelectionRoot; 3] = [
        ExportSelectionRoot::Policies,
        ExportSelectionRoot::Shaders,
        ExportSelectionRoot::Playlists,
    ];

    fn label(self) -> &'static str {
        match self {
            ExportSelectionRoot::Policies => "Policies",
            ExportSelectionRoot::Shaders => "Shaders",
            ExportSelectionRoot::Playlists => "Playlists",
        }
    }

    fn select_label(self) -> &'static str {
        match self {
            ExportSelectionRoot::Policies => "Select Policies",
            ExportSelectionRoot::Shaders => "Select Shaders",
            ExportSelectionRoot::Playlists => "Select Playlists",
        }
    }

    fn tab_order(self) -> [SelectDataTab; 3] {
        match self {
            ExportSelectionRoot::Policies => [
                SelectDataTab::Policies,
                SelectDataTab::Shaders,
                SelectDataTab::Playlists,
            ],
            ExportSelectionRoot::Shaders => [
                SelectDataTab::Shaders,
                SelectDataTab::Policies,
                SelectDataTab::Playlists,
            ],
            ExportSelectionRoot::Playlists => [
                SelectDataTab::Playlists,
                SelectDataTab::Policies,
                SelectDataTab::Shaders,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectDataTab {
    Policies,
    Shaders,
    Playlists,
}

impl SelectDataTab {
    fn label(self) -> &'static str {
        match self {
            SelectDataTab::Policies => "Policies",
            SelectDataTab::Shaders => "Shaders",
            SelectDataTab::Playlists => "Playlists",
        }
    }
}

impl ExportStage {
    fn label(self) -> &'static str {
        match self {
            ExportStage::SelectFocus => "Select Export Focus",
            ExportStage::SelectData => "Select Data",
            ExportStage::Destination => "Destination",
            ExportStage::Review => "Review & Confirm",
            ExportStage::Results => "Results",
        }
    }
}

#[derive(Clone, Debug)]
struct ExportPolicyChoice {
    policy_id: i64,
    policy_name: String,
    policy_target: String,
    shader_id: i64,
    shader_filename: String,
    shader_source_path: String,
    playlists: Vec<(i64, String)>,
}

#[derive(Clone, Debug)]
struct ExportShaderChoice {
    shader_id: i64,
    filename: String,
    source_path: String,
}

#[derive(Clone, Debug)]
struct ExportPlaylistChoice {
    playlist_id: i64,
    playlist_name: String,
    description: Option<String>,
}

#[derive(Clone, Debug)]
enum PortableTextureSelection {
    Specific {
        family: String,
        primitives: i64,
    },
    Random {
        primitives: Option<i64>,
    },
    InheritTarget,
}

#[derive(Clone, Debug)]
enum PortablePaletteSelection {
    Specific(String),
    Random,
    InheritTarget,
}

#[derive(Clone, Debug)]
enum PortableTargetValue<T> {
    Explicit(T),
    InheritTarget,
}

#[derive(Clone, Debug)]
struct PortablePolicy {
    policy_id: i64,
    policy_name: String,
    policy_target: String,
    shader_id: i64,
    shader_filename: String,
    shader_source_path: String,
    texture: PortableTextureSelection,
    palette: PortablePaletteSelection,
    rendered_fps: i64,
    animation_speed: PortableTargetValue<f64>,
    starting_offset: f64,
    anti_aliasing: String,
    dithering: String,
    color_precision: String,
    render_scale: f64,
    audiovisual_effect: String,
    bloom_intensity: f64,
    bloom_saturation: f64,
    bloom_threshold: f64,
    bloom_frequency_rotation: f64,
    bloom_frequency_invert: bool,
    invert_colors: bool,
    flip_horizontal: bool,
    flip_vertical: bool,
    hue_rotation: f64,
}

#[derive(Clone, Debug)]
struct ExportWizardState {
    open: bool,
    stage: ExportStage,
    export_focus: ExportSelectionRoot,
    select_data_tab: SelectDataTab,
    policies: Vec<ExportPolicyChoice>,
    shaders: Vec<ExportShaderChoice>,
    playlists: Vec<ExportPlaylistChoice>,
    selected_policy_ids: std::collections::HashSet<i64>,
    selected_shader_ids: std::collections::HashSet<i64>,
    selected_playlist_ids: std::collections::HashSet<i64>,
    selection_error: Option<String>,
    portable_policies: Vec<PortablePolicy>,
    portable_policy_error: Option<String>,
    destination: String,
    export_filename: String,
    execution_started: bool,
    export_result: Option<Result<ExportSuccess, String>>,
}

impl Default for ExportWizardState {
    fn default() -> Self {
        Self {
            open: false,
            stage: ExportStage::SelectFocus,
            export_focus: ExportSelectionRoot::Policies,
            select_data_tab: SelectDataTab::Policies,
            policies: Vec::new(),
            shaders: Vec::new(),
            playlists: Vec::new(),
            selected_policy_ids: std::collections::HashSet::new(),
            selected_shader_ids: std::collections::HashSet::new(),
            selected_playlist_ids: std::collections::HashSet::new(),
            selection_error: None,
            portable_policies: Vec::new(),
            portable_policy_error: None,
            destination: default_export_destination_folder(),
            export_filename: default_export_filename(),
            execution_started: false,
            export_result: None,
        }
    }
}

impl ExportWizardState {
    fn focus_selection_nonempty(&self) -> bool {
        match self.export_focus {
            ExportSelectionRoot::Policies => !self.selected_policy_ids.is_empty(),
            ExportSelectionRoot::Shaders => !self.selected_shader_ids.is_empty(),
            ExportSelectionRoot::Playlists => !self.selected_playlist_ids.is_empty(),
        }
    }

    fn select_data_valid(&self) -> bool {
        self.selection_error.is_none()
            && self.portable_policy_error.is_none()
            && self.focus_selection_nonempty()
    }

    fn destination_valid(&self) -> bool {
        !self.destination.trim().is_empty()
            && valid_export_filename(&self.export_filename)
    }

    fn resolved_export_path(&self) -> Option<std::path::PathBuf> {
        if !self.destination_valid() {
            return None;
        }

        Some(
            collision_safe_export_path(
                std::path::Path::new(self.destination.trim()),
                self.export_filename.trim(),
            )
        )
    }

    fn stage_enabled(&self, stage: ExportStage) -> bool {
        match stage {
            ExportStage::SelectFocus => !self.execution_started,
            ExportStage::SelectData => !self.execution_started,
            ExportStage::Destination => {
                !self.execution_started
                    && self.select_data_valid()
            }
            ExportStage::Review => {
                !self.execution_started
                    && self.select_data_valid()
                    && self.destination_valid()
            }
            ExportStage::Results => self.execution_started,
        }
    }

    fn reset_for_open(&mut self) {
        let (policies, selection_error) =
            match load_export_policy_choices() {
                Ok(policies) => (policies, None),
                Err(error) => (
                    Vec::new(),
                    Some(error),
                ),
            };

        let shaders = export_shader_choices_from_policies(&policies);

        let playlists =
            if selection_error.is_none() {
                match load_export_playlist_choices() {
                    Ok(playlists) => playlists,
                    Err(_error) => Vec::new(),
                }
            } else {
                Vec::new()
            };

        let selected_policy_ids =
            policies
                .iter()
                .map(|policy| policy.policy_id)
                .collect();

        let selected_shader_ids =
            shaders
                .iter()
                .map(|shader| shader.shader_id)
                .collect();

        let selected_playlist_ids =
            playlists
                .iter()
                .map(|playlist| playlist.playlist_id)
                .collect();

        *self = Self {
            open: true,
            stage: ExportStage::SelectFocus,
            export_focus: ExportSelectionRoot::Policies,
            select_data_tab: SelectDataTab::Policies,
            policies,
            shaders,
            playlists,
            selected_policy_ids,
            selected_shader_ids,
            selected_playlist_ids,
            selection_error,
            portable_policies: Vec::new(),
            portable_policy_error: None,
            destination: default_export_destination_folder(),
            export_filename: default_export_filename(),
            execution_started: false,
            export_result: None,
        };

        self.refresh_portable_policies();
    }

    fn set_export_focus(&mut self, focus: ExportSelectionRoot) {
        if self.export_focus != focus {
            self.export_focus = focus;
            self.select_data_tab = focus.tab_order()[0];
            self.refresh_portable_policies();
        }
    }

    fn included_policy_ids(&self) -> std::collections::HashSet<i64> {
        match self.export_focus {
            ExportSelectionRoot::Policies => self.selected_policy_ids.clone(),

            ExportSelectionRoot::Shaders => {
                self.policies
                    .iter()
                    .filter(
                        |policy| {
                            self.selected_shader_ids.contains(
                                &policy.shader_id
                            )
                        }
                    )
                    .map(|policy| policy.policy_id)
                    .collect()
            }

            ExportSelectionRoot::Playlists => {
                self.policies
                    .iter()
                    .filter(
                        |policy| {
                            policy.playlists.iter().any(
                                |(playlist_id, _)| {
                                    self.selected_playlist_ids.contains(
                                        playlist_id
                                    )
                                }
                            )
                        }
                    )
                    .map(|policy| policy.policy_id)
                    .collect()
            }
        }
    }

    fn refresh_portable_policies(&mut self) {
        let included_policy_ids = self.included_policy_ids();

        if self.selection_error.is_some()
            || !self.focus_selection_nonempty()
        {
            self.portable_policies.clear();
            self.portable_policy_error = None;
            return;
        }

        match resolve_portable_policies(
            &included_policy_ids
        ) {
            Ok(portable_policies) => {
                self.portable_policies = portable_policies;
                self.portable_policy_error = None;
            }

            Err(error) => {
                self.portable_policies.clear();
                self.portable_policy_error = Some(error);
            }
        }
    }

    fn selected_focus_count(&self) -> usize {
        match self.export_focus {
            ExportSelectionRoot::Policies => self.selected_policy_ids.len(),
            ExportSelectionRoot::Shaders => self.selected_shader_ids.len(),
            ExportSelectionRoot::Playlists => self.selected_playlist_ids.len(),
        }
    }

    fn focus_total_count(&self) -> usize {
        match self.export_focus {
            ExportSelectionRoot::Policies => self.policies.len(),
            ExportSelectionRoot::Shaders => self.shaders.len(),
            ExportSelectionRoot::Playlists => self.playlists.len(),
        }
    }

    fn included_policies(&self) -> Vec<&ExportPolicyChoice> {
        let ids = self.included_policy_ids();

        self.policies
            .iter()
            .filter(|policy| ids.contains(&policy.policy_id))
            .collect()
    }

    fn included_shaders(&self) -> Vec<(i64, String, String)> {
        if self.export_focus == ExportSelectionRoot::Shaders {
            return self.shaders
                .iter()
                .filter(
                    |shader| {
                        self.selected_shader_ids.contains(
                            &shader.shader_id
                        )
                    }
                )
                .map(
                    |shader| {
                        (
                            shader.shader_id,
                            shader.filename.clone(),
                            shader.source_path.clone(),
                        )
                    }
                )
                .collect();
        }

        let included_policy_ids = self.included_policy_ids();
        let mut shaders =
            std::collections::BTreeMap::<
                i64,
                (String, String),
            >::new();

        for policy in &self.policies {
            if included_policy_ids.contains(
                &policy.policy_id
            ) {
                shaders
                    .entry(policy.shader_id)
                    .or_insert_with(
                        || {
                            (
                                policy.shader_filename.clone(),
                                policy.shader_source_path.clone(),
                            )
                        }
                    );
            }
        }

        shaders
            .into_iter()
            .map(
                |(shader_id, (filename, source_path))| {
                    (
                        shader_id,
                        filename,
                        source_path,
                    )
                }
            )
            .collect()
    }

    fn included_playlists(&self) -> Vec<(i64, String)> {
        if self.export_focus == ExportSelectionRoot::Playlists {
            return self.playlists
                .iter()
                .filter(
                    |playlist| {
                        self.selected_playlist_ids.contains(
                            &playlist.playlist_id
                        )
                    }
                )
                .map(
                    |playlist| {
                        (
                            playlist.playlist_id,
                            playlist.playlist_name.clone(),
                        )
                    }
                )
                .collect();
        }

        let included_policy_ids = self.included_policy_ids();
        let mut playlists =
            std::collections::BTreeMap::<i64, String>::new();

        for policy in &self.policies {
            if included_policy_ids.contains(
                &policy.policy_id
            ) {
                for (playlist_id, playlist_name)
                    in &policy.playlists
                {
                    playlists
                        .entry(*playlist_id)
                        .or_insert_with(
                            || playlist_name.clone()
                        );
                }
            }
        }

        playlists.into_iter().collect()
    }
}

fn export_shader_choices_from_policies(
    policies: &[ExportPolicyChoice],
) -> Vec<ExportShaderChoice> {
    let mut shaders =
        std::collections::BTreeMap::<
            i64,
            ExportShaderChoice,
        >::new();

    for policy in policies {
        shaders
            .entry(policy.shader_id)
            .or_insert_with(
                || ExportShaderChoice {
                    shader_id: policy.shader_id,
                    filename: policy.shader_filename.clone(),
                    source_path: policy.shader_source_path.clone(),
                }
            );
    }

    let mut shaders =
        shaders.into_values().collect::<Vec<_>>();

    shaders.sort_by(
        |left, right| {
            left.filename
                .to_lowercase()
                .cmp(&right.filename.to_lowercase())
                .then_with(|| left.filename.cmp(&right.filename))
                .then_with(|| left.shader_id.cmp(&right.shader_id))
        }
    );

    shaders
}

fn load_export_playlist_choices(
) -> Result<Vec<ExportPlaylistChoice>, String> {
    crate::manage_playlists::list_playlists()
        .map(
            |playlists| {
                playlists
                    .into_iter()
                    .map(
                        |playlist| ExportPlaylistChoice {
                            playlist_id: playlist.playlist_id,
                            playlist_name: playlist.playlist_name,
                            description: playlist.description,
                        }
                    )
                    .collect()
            }
        )
        .map_err(
            |error| {
                format!(
                    "Unable to load playlists for export selection: {}",
                    error,
                )
            }
        )
}



#[derive(Clone, Debug)]
struct ExportSuccess {
    path: std::path::PathBuf,
    policy_count: usize,
    shader_count: usize,
    playlist_count: usize,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExportSchemaArchive {
    manifest: String,
    shader_directory: String,
    metadata_files: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExportSchemaIntegrity {
    algorithm: String,
    package_canonicalization: String,
    package_hash_excludes: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExportSchemaDataset {
    file: String,
    columns: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExportSchemaDatasets {
    policies: ExportSchemaDataset,
    shaders: ExportSchemaDataset,
    playlists: ExportSchemaDataset,
    playlist_members: ExportSchemaDataset,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExportSchema {
    format: String,
    format_version: u32,
    archive: ExportSchemaArchive,
    integrity: ExportSchemaIntegrity,
    datasets: ExportSchemaDatasets,
}

fn export_schema() -> Result<ExportSchema, String> {
    let schema: ExportSchema =
        serde_json::from_str(
            include_str!("../assets/export/schema_v001.json")
        )
        .map_err(
            |error| {
                format!(
                    "Unable to load Screenshaver Export Schema V1: {}",
                    error,
                )
            }
        )?;

    if schema.format.trim().is_empty() {
        return Err(
            "Screenshaver Export Schema V1 has an empty format identifier."
                .to_string()
        );
    }

    if schema.format_version == 0 {
        return Err(
            "Screenshaver Export Schema V1 has an invalid format version."
                .to_string()
        );
    }

    if schema.integrity.algorithm != "sha256"
        || schema.integrity.package_canonicalization
            != "screenshaver-package-v1"
    {
        return Err(
            "Screenshaver Export Schema V1 requests an unsupported integrity algorithm or canonicalization."
                .to_string()
        );
    }

    if !schema
        .integrity
        .package_hash_excludes
        .iter()
        .any(|name| name == &schema.archive.manifest)
    {
        return Err(
            "Screenshaver Export Schema V1 must exclude its manifest from the package hash."
                .to_string()
        );
    }

    let expected_metadata = [
        &schema.datasets.policies.file,
        &schema.datasets.shaders.file,
        &schema.datasets.playlists.file,
        &schema.datasets.playlist_members.file,
    ];

    for file in expected_metadata {
        if !schema.archive.metadata_files.iter().any(|name| name == file) {
            return Err(
                format!(
                    "Screenshaver Export Schema V1 dataset '{}' is not declared as archive metadata.",
                    file,
                )
            );
        }
    }

    Ok(schema)
}

fn schema_tsv_header(
    dataset: &ExportSchemaDataset,
) -> String {
    let mut header = dataset.columns.join("\t");
    header.push('\n');
    header
}

#[derive(serde::Serialize)]
struct ExportManifestFileIntegrity {
    sha256: String,
}

type ExportManifestFiles =
    std::collections::BTreeMap<String, ExportManifestFileIntegrity>;

#[derive(serde::Serialize)]
struct ExportManifest {
    format: String,
    format_version: u32,
    screenshaver_version: &'static str,
    database_schema_version: u32,
    created: String,
    export_focus: String,
    policy_count: usize,
    shader_count: usize,
    playlist_count: usize,
    package_sha256: String,
    files: ExportManifestFiles,
}

const DATABASE_SCHEMA_VERSION: u32 = 1;

fn export_created_timestamp() -> String {
    std::process::Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%dT%H:%M:%SZ")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn tsv_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn portable_texture_fields(
    texture: &PortableTextureSelection,
) -> (String, String, String) {
    match texture {
        PortableTextureSelection::Specific {
            family,
            primitives,
        } => (
            "specific".to_string(),
            family.clone(),
            primitives.to_string(),
        ),
        PortableTextureSelection::Random {
            primitives,
        } => (
            "random".to_string(),
            String::new(),
            primitives
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        PortableTextureSelection::InheritTarget => (
            "inherit_target".to_string(),
            String::new(),
            String::new(),
        ),
    }
}

fn portable_palette_fields(
    palette: &PortablePaletteSelection,
) -> (String, String) {
    match palette {
        PortablePaletteSelection::Specific(color) => (
            "specific".to_string(),
            color.clone(),
        ),
        PortablePaletteSelection::Random => (
            "random".to_string(),
            String::new(),
        ),
        PortablePaletteSelection::InheritTarget => (
            "inherit_target".to_string(),
            String::new(),
        ),
    }
}

fn portable_target_f64(
    value: &PortableTargetValue<f64>,
) -> (String, String) {
    match value {
        PortableTargetValue::Explicit(value) => (
            "explicit".to_string(),
            value.to_string(),
        ),
        PortableTargetValue::InheritTarget => (
            "inherit_target".to_string(),
            String::new(),
        ),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;

    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

fn package_sha256(
    payloads: &[(&str, &[u8])],
) -> String {
    use sha2::Digest;

    // The package fingerprint is deliberately independent of ZIP container
    // metadata (timestamps, compression details, entry ordering). It covers
    // every defined payload member by hashing a canonical sequence of:
    //
    //   UTF-8 archive member name
    //   NUL separator
    //   decimal byte length
    //   NUL separator
    //   SHA-256(payload bytes) as lowercase hexadecimal
    //   newline
    //
    // Members are sorted by archive name before hashing. manifest.json is not
    // included because it contains this resulting package_sha256 value.
    let mut members = payloads
        .iter()
        .map(|(name, bytes)| {
            (
                (*name).to_string(),
                bytes.len(),
                sha256_hex(bytes),
            )
        })
        .collect::<Vec<_>>();

    members.sort_by(|left, right| left.0.cmp(&right.0));

    let mut canonical = Vec::<u8>::new();

    for (name, length, digest) in members {
        canonical.extend_from_slice(name.as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(length.to_string().as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(digest.as_bytes());
        canonical.push(b'\n');
    }

    sha256_hex(&canonical)
}

fn package_id_map(
    source_ids: impl IntoIterator<Item = i64>,
) -> std::collections::HashMap<i64, i64> {
    let mut ids = source_ids.into_iter().collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();

    ids.into_iter()
        .enumerate()
        .map(|(index, source_id)| (source_id, index as i64 + 1))
        .collect()
}

fn build_policies_tsv(
    dataset: &ExportSchemaDataset,
    policies: &[PortablePolicy],
    policy_export_ids: &std::collections::HashMap<i64, i64>,
    shader_export_ids: &std::collections::HashMap<i64, i64>,
) -> Result<String, String> {
    let mut output = schema_tsv_header(dataset);

    for policy in policies {
        let (texture_mode, texture_family, texture_primitives) =
            portable_texture_fields(&policy.texture);
        let (palette_mode, palette_color) =
            portable_palette_fields(&policy.palette);
        let (animation_speed_mode, animation_speed) =
            portable_target_f64(&policy.animation_speed);

        let policy_export_id = policy_export_ids
            .get(&policy.policy_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Missing package-local export ID for policy '{}'",
                    policy.policy_name,
                )
            })?;

        let shader_export_id = shader_export_ids
            .get(&policy.shader_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Missing package-local export ID for shader '{}' referenced by policy '{}'",
                    policy.shader_filename,
                    policy.policy_name,
                )
            })?;

        let fields = vec![
            policy_export_id.to_string(),
            tsv_field(&policy.policy_name),
            tsv_field(&policy.policy_target),
            shader_export_id.to_string(),
            tsv_field(&policy.shader_filename),
            tsv_field(&texture_mode),
            tsv_field(&texture_family),
            texture_primitives,
            tsv_field(&palette_mode),
            tsv_field(&palette_color),
            policy.rendered_fps.to_string(),
            animation_speed_mode,
            animation_speed,
            policy.starting_offset.to_string(),
            tsv_field(&policy.anti_aliasing),
            tsv_field(&policy.dithering),
            tsv_field(&policy.color_precision),
            policy.render_scale.to_string(),
            tsv_field(&policy.audiovisual_effect),
            policy.bloom_intensity.to_string(),
            policy.bloom_saturation.to_string(),
            policy.bloom_threshold.to_string(),
            policy.bloom_frequency_rotation.to_string(),
            policy.bloom_frequency_invert.to_string(),
            policy.invert_colors.to_string(),
            policy.flip_horizontal.to_string(),
            policy.flip_vertical.to_string(),
            policy.hue_rotation.to_string(),
        ];

        output.push_str(&fields.join("\t"));
        output.push('\n');
    }

    Ok(output)
}

fn build_playlists_tsv(
    dataset: &ExportSchemaDataset,
    playlists: &[ExportPlaylistChoice],
    playlist_export_ids: &std::collections::HashMap<i64, i64>,
) -> Result<String, String> {
    let mut output = schema_tsv_header(dataset);

    for playlist in playlists {
        let playlist_export_id = playlist_export_ids
            .get(&playlist.playlist_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Missing package-local export ID for playlist '{}'",
                    playlist.playlist_name,
                )
            })?;

        output.push_str(
            &format!(
                "{}\t{}\t{}\n",
                playlist_export_id,
                tsv_field(&playlist.playlist_name),
                tsv_field(playlist.description.as_deref().unwrap_or("")),
            )
        );
    }

    Ok(output)
}

fn build_playlist_members_tsv(
    dataset: &ExportSchemaDataset,
    playlists: &[ExportPlaylistChoice],
    included_policy_ids: &std::collections::HashSet<i64>,
    playlist_export_ids: &std::collections::HashMap<i64, i64>,
    policy_export_ids: &std::collections::HashMap<i64, i64>,
) -> Result<String, String> {
    let mut output = schema_tsv_header(dataset);

    for playlist in playlists {
        let playlist_export_id = playlist_export_ids
            .get(&playlist.playlist_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Missing package-local export ID for playlist '{}'",
                    playlist.playlist_name,
                )
            })?;

        let members =
            crate::manage_playlists::playlist_members(playlist.playlist_id)
                .map_err(
                    |error| {
                        format!(
                            "Unable to load members for playlist {} while exporting: {}",
                            playlist.playlist_id,
                            error,
                        )
                    }
                )?;

        let mut portable_position = 1_i64;

        for member in members {
            if included_policy_ids.contains(&member.policy_id) {
                let policy_export_id = policy_export_ids
                    .get(&member.policy_id)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "Missing package-local export ID for policy {} in playlist '{}'",
                            member.policy_id,
                            playlist.playlist_name,
                        )
                    })?;

                output.push_str(
                    &format!(
                        "{}\t{}\t{}\n",
                        playlist_export_id,
                        policy_export_id,
                        portable_position,
                    )
                );
                portable_position += 1;
            }
        }
    }

    Ok(output)
}

fn shader_archive_name(
    shader_directory: &str,
    shader_id: i64,
    filename: &str,
) -> String {
    let safe_filename =
        std::path::Path::new(filename)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("shader.glsl");

    format!(
        "{}/{}-{}",
        shader_directory.trim_end_matches('/'),
        shader_id,
        safe_filename,
    )
}

fn build_shaders_tsv(
    dataset: &ExportSchemaDataset,
    shader_directory: &str,
    shaders: &[(i64, String, String)],
    shader_export_ids: &std::collections::HashMap<i64, i64>,
) -> Result<(String, Vec<(String, std::path::PathBuf)>), String> {
    use sha2::Digest;

    let mut output = schema_tsv_header(dataset);
    let mut files = Vec::new();

    for (shader_id, filename, source_path) in shaders {
        // shaders.source_path stores the directory containing the
        // physical shader; shaders.filename stores the filename.
        let path =
            std::path::PathBuf::from(source_path)
                .join(filename);

        let bytes =
            std::fs::read(&path)
                .map_err(
                    |error| {
                        format!(
                            "Unable to read shader '{}' at '{}': {}",
                            filename,
                            path.display(),
                            error,
                        )
                    }
                )?;

        let digest = sha2::Sha256::digest(&bytes);
        let checksum =
            digest
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>();

        let shader_export_id = shader_export_ids
            .get(shader_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Missing package-local export ID for shader '{}'",
                    filename,
                )
            })?;

        let archive_name =
            shader_archive_name(shader_directory, shader_export_id, filename);

        output.push_str(
            &format!(
                "{}\t{}\t{}\t{}\n",
                shader_export_id,
                tsv_field(filename),
                tsv_field(&archive_name),
                checksum,
            )
        );

        files.push((archive_name, path));
    }

    Ok((output, files))
}

fn write_zip_text(
    zip: &mut zip::ZipWriter<std::fs::File>,
    name: &str,
    contents: &str,
) -> Result<(), String> {
    use std::io::Write;

    let options =
        zip::write::SimpleFileOptions::default()
            .compression_method(
                zip::CompressionMethod::Deflated
            );

    zip.start_file(name, options)
        .map_err(
            |error| {
                format!(
                    "Unable to create '{}' in export archive: {}",
                    name,
                    error,
                )
            }
        )?;

    zip.write_all(contents.as_bytes())
        .map_err(
            |error| {
                format!(
                    "Unable to write '{}' to export archive: {}",
                    name,
                    error,
                )
            }
        )
}

fn create_export_archive(
    state: &ExportWizardState,
) -> Result<ExportSuccess, String> {
    use std::io::{Read, Write};

    let schema = export_schema()?;

    let final_path =
        state.resolved_export_path()
            .ok_or_else(
                || "The export destination is not valid.".to_string()
            )?;

    let destination_folder =
        final_path.parent()
            .ok_or_else(
                || "The export destination folder is not valid.".to_string()
            )?;

    if !destination_folder.is_dir() {
        return Err(
            format!(
                "Export destination folder does not exist: {}",
                destination_folder.display(),
            )
        );
    }

    let included_policy_ids =
        state.included_policy_ids();
    let shaders =
        state.included_shaders();
    let included_playlist_ids =
        state.included_playlists()
            .into_iter()
            .map(|(playlist_id, _)| playlist_id)
            .collect::<std::collections::HashSet<_>>();
    let playlists =
        state.playlists
            .iter()
            .filter(|playlist| {
                included_playlist_ids.contains(&playlist.playlist_id)
            })
            .cloned()
            .collect::<Vec<_>>();

    let policy_export_ids =
        package_id_map(included_policy_ids.iter().copied());
    let shader_export_ids =
        package_id_map(shaders.iter().map(|(shader_id, _, _)| *shader_id));
    let playlist_export_ids =
        package_id_map(playlists.iter().map(|playlist| playlist.playlist_id));

    if state.portable_policies.len()
        != included_policy_ids.len()
    {
        return Err(
            "The effective policy snapshot is incomplete. Return to selection and try again."
                .to_string()
        );
    }

    let policies_tsv =
        build_policies_tsv(
            &schema.datasets.policies,
            &state.portable_policies,
            &policy_export_ids,
            &shader_export_ids,
        )?;
    let (shaders_tsv, shader_files) =
        build_shaders_tsv(
            &schema.datasets.shaders,
            &schema.archive.shader_directory,
            &shaders,
            &shader_export_ids,
        )?;
    let playlists_tsv =
        build_playlists_tsv(
            &schema.datasets.playlists,
            &playlists,
            &playlist_export_ids,
        )?;
    let playlist_members_tsv =
        build_playlist_members_tsv(
            &schema.datasets.playlist_members,
            &playlists,
            &included_policy_ids,
            &playlist_export_ids,
            &policy_export_ids,
        )?;

    let mut package_payloads =
        vec![
            (schema.datasets.policies.file.as_str(), policies_tsv.as_bytes()),
            (schema.datasets.shaders.file.as_str(), shaders_tsv.as_bytes()),
            (schema.datasets.playlists.file.as_str(), playlists_tsv.as_bytes()),
            (schema.datasets.playlist_members.file.as_str(), playlist_members_tsv.as_bytes()),
        ];

    let shader_payload_bytes =
        shader_files
            .iter()
            .map(
                |(archive_name, source_path)| {
                    std::fs::read(source_path)
                        .map(|bytes| (archive_name.clone(), bytes))
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to read shader '{}' while calculating package integrity: {}",
                                    source_path.display(),
                                    error,
                                )
                            }
                        )
                }
            )
            .collect::<Result<Vec<_>, String>>()?;

    for (archive_name, bytes) in &shader_payload_bytes {
        package_payloads.push(
            (archive_name.as_str(), bytes.as_slice())
        );
    }

    let package_sha256 =
        package_sha256(&package_payloads);

    let mut manifest_files = ExportManifestFiles::new();
    manifest_files.insert(
        schema.datasets.policies.file.clone(),
        ExportManifestFileIntegrity {
            sha256: sha256_hex(policies_tsv.as_bytes()),
        },
    );
    manifest_files.insert(
        schema.datasets.shaders.file.clone(),
        ExportManifestFileIntegrity {
            sha256: sha256_hex(shaders_tsv.as_bytes()),
        },
    );
    manifest_files.insert(
        schema.datasets.playlists.file.clone(),
        ExportManifestFileIntegrity {
            sha256: sha256_hex(playlists_tsv.as_bytes()),
        },
    );
    manifest_files.insert(
        schema.datasets.playlist_members.file.clone(),
        ExportManifestFileIntegrity {
            sha256: sha256_hex(playlist_members_tsv.as_bytes()),
        },
    );

    let manifest =
        ExportManifest {
            format: schema.format.clone(),
            format_version:
                schema.format_version,
            screenshaver_version:
                env!("CARGO_PKG_VERSION"),
            database_schema_version:
                DATABASE_SCHEMA_VERSION,
            created:
                export_created_timestamp(),
            export_focus:
                state.export_focus.label().to_string(),
            policy_count:
                state.portable_policies.len(),
            shader_count:
                shaders.len(),
            playlist_count:
                playlists.len(),
            package_sha256,
            files: manifest_files,
        };

    let manifest_json =
        serde_json::to_string_pretty(&manifest)
            .map_err(
                |error| {
                    format!(
                        "Unable to serialize export manifest: {}",
                        error,
                    )
                }
            )?;

    let temporary_path =
        final_path.with_extension("zip.part");

    if temporary_path.exists() {
        std::fs::remove_file(&temporary_path)
            .map_err(
                |error| {
                    format!(
                        "Unable to remove stale temporary export '{}': {}",
                        temporary_path.display(),
                        error,
                    )
                }
            )?;
    }

    let file =
        std::fs::File::create(&temporary_path)
            .map_err(
                |error| {
                    format!(
                        "Unable to create temporary export archive '{}': {}",
                        temporary_path.display(),
                        error,
                    )
                }
            )?;

    let mut zip =
        zip::ZipWriter::new(file);

    let write_result =
        (|| -> Result<(), String> {
            write_zip_text(
                &mut zip,
                &schema.archive.manifest,
                &manifest_json,
            )?;
            write_zip_text(
                &mut zip,
                &schema.datasets.policies.file,
                &policies_tsv,
            )?;
            write_zip_text(
                &mut zip,
                &schema.datasets.shaders.file,
                &shaders_tsv,
            )?;
            write_zip_text(
                &mut zip,
                &schema.datasets.playlists.file,
                &playlists_tsv,
            )?;
            write_zip_text(
                &mut zip,
                &schema.datasets.playlist_members.file,
                &playlist_members_tsv,
            )?;

            let options =
                zip::write::SimpleFileOptions::default()
                    .compression_method(
                        zip::CompressionMethod::Deflated
                    );

            let mut buffer = [0_u8; 64 * 1024];

            for (archive_name, source_path)
                in &shader_files
            {
                zip.start_file(
                    archive_name,
                    options,
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to add shader '{}' to export archive: {}",
                            source_path.display(),
                            error,
                        )
                    }
                )?;

                let mut source =
                    std::fs::File::open(source_path)
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to open shader '{}': {}",
                                    source_path.display(),
                                    error,
                                )
                            }
                        )?;

                loop {
                    let read =
                        source.read(&mut buffer)
                            .map_err(
                                |error| {
                                    format!(
                                        "Unable to read shader '{}': {}",
                                        source_path.display(),
                                        error,
                                    )
                                }
                            )?;

                    if read == 0 {
                        break;
                    }

                    zip.write_all(&buffer[..read])
                        .map_err(
                            |error| {
                                format!(
                                    "Unable to write shader '{}' to export archive: {}",
                                    source_path.display(),
                                    error,
                                )
                            }
                        )?;
                }
            }

            Ok(())
        })();

    if let Err(error) = write_result {
        drop(zip);
        let _ =
            std::fs::remove_file(&temporary_path);
        return Err(error);
    }

    zip.finish()
        .map_err(
            |error| {
                let _ =
                    std::fs::remove_file(&temporary_path);
                format!(
                    "Unable to finalize export archive: {}",
                    error,
                )
            }
        )?;

    std::fs::rename(
        &temporary_path,
        &final_path,
    )
    .map_err(
        |error| {
            let _ =
                std::fs::remove_file(&temporary_path);
            format!(
                "Unable to move completed export archive to '{}': {}",
                final_path.display(),
                error,
            )
        }
    )?;

    Ok(
        ExportSuccess {
            path: final_path,
            policy_count:
                state.portable_policies.len(),
            shader_count:
                shaders.len(),
            playlist_count:
                playlists.len(),
        }
    )
}

fn default_export_destination_folder() -> String {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(
            || std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
        )
        .to_string_lossy()
        .to_string()
}

fn default_export_filename() -> String {
    let timestamp =
        std::process::Command::new("date")
            .arg("+%Y-%m-%d-%H%M%S")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(
                |output| {
                    String::from_utf8(output.stdout)
                        .ok()
                }
            )
            .map(
                |value| value.trim().to_string()
            )
            .filter(|value| !value.is_empty())
            .unwrap_or_else(
                || "timestamp".to_string()
            );

    format!(
        "Screenshaver-Export-{}.zip",
        timestamp,
    )
}

fn valid_export_filename(
    filename: &str,
) -> bool {
    let filename = filename.trim();

    !filename.is_empty()
        && filename != "."
        && filename != ".."
        && !filename.contains('/')
        && !filename.contains('\\')
        && !filename.contains('\0')
}

fn collision_safe_export_path(
    destination_folder: &std::path::Path,
    requested_filename: &str,
) -> std::path::PathBuf {
    let requested_filename =
        requested_filename.trim();

    let requested_path =
        destination_folder.join(
            requested_filename
        );

    if !requested_path.exists() {
        return requested_path;
    }

    let requested =
        std::path::Path::new(
            requested_filename
        );

    let stem =
        requested.file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(requested_filename);

    let extension =
        requested.extension()
            .and_then(|value| value.to_str());

    for ordinal in 2_u32..=10_000 {
        let candidate_name =
            match extension {
                Some(extension)
                    if !extension.is_empty() =>
                {
                    format!(
                        "{} ({}).{}",
                        stem,
                        ordinal,
                        extension,
                    )
                }

                _ => {
                    format!(
                        "{} ({})",
                        stem,
                        ordinal,
                    )
                }
            };

        let candidate_path =
            destination_folder.join(
                candidate_name
            );

        if !candidate_path.exists() {
            return candidate_path;
        }
    }

    requested_path
}

fn resolve_portable_policies(
    selected_policy_ids: &std::collections::HashSet<i64>,
) -> Result<Vec<PortablePolicy>, String> {
    let app_defaults =
        crate::manage_configuration::load_app_defaults()?;

    let screensaver_defaults =
        crate::manage_configuration::load_target_defaults(
            "screensaver"
        )?;

    let wallpaper_defaults =
        crate::manage_configuration::load_target_defaults(
            "wallpaper"
        )?;

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open screenshaver.db while resolving export policies: {}",
                        error,
                    )
                }
            )?;

    let mut statement =
        connection
            .prepare(
                "SELECT
                     p.policy_id,
                     p.policy_name,
                     p.policy_target,
                     s.shader_id,
                     s.filename,
                     s.source_path,
                     p.texture_mode,
                     p.texture_family,
                     p.texture_primitives,
                     p.palette_mode,
                     p.palette_color,
                     p.rendered_fps,
                     p.animation_speed,
                     p.starting_offset,
                     p.anti_aliasing,
                     p.dithering,
                     p.color_precision,
                     p.render_scale,
                     p.audiovisual_effect,
                     p.bloom_intensity,
                     p.bloom_saturation,
                     p.bloom_threshold,
                     p.bloom_frequency_rotation,
                     p.bloom_frequency_invert,
                     p.invert_colors,
                     p.flip_horizontal,
                     p.flip_vertical,
                     p.hue_rotation
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_id = ?1"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare effective export policy query: {}",
                        error,
                    )
                }
            )?;

    let mut policy_ids =
        selected_policy_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();

    policy_ids.sort_unstable();

    let mut portable_policies = Vec::new();

    for policy_id in policy_ids {
        let row =
            statement
                .query_row(
                    [policy_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<i64>>(8)?,
                            row.get::<_, Option<String>>(9)?,
                            row.get::<_, Option<String>>(10)?,
                            row.get::<_, Option<i64>>(11)?,
                            row.get::<_, Option<f64>>(12)?,
                            row.get::<_, f64>(13)?,
                            row.get::<_, Option<String>>(14)?,
                            row.get::<_, Option<String>>(15)?,
                            row.get::<_, Option<String>>(16)?,
                            row.get::<_, Option<f64>>(17)?,
                            row.get::<_, String>(18)?,
                            row.get::<_, f64>(19)?,
                            row.get::<_, f64>(20)?,
                            row.get::<_, f64>(21)?,
                            row.get::<_, f64>(22)?,
                            row.get::<_, i64>(23)?,
                            row.get::<_, i64>(24)?,
                            row.get::<_, i64>(25)?,
                            row.get::<_, i64>(26)?,
                            row.get::<_, f64>(27)?,
                        ))
                    },
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to read selected policy ID {} while resolving export data: {}",
                            policy_id,
                            error,
                        )
                    }
                )?;

        let (
            policy_id,
            policy_name,
            policy_target,
            shader_id,
            shader_filename,
            shader_source_path,
            texture_mode,
            texture_family,
            texture_primitives,
            palette_mode,
            palette_color,
            rendered_fps,
            animation_speed,
            starting_offset,
            anti_aliasing,
            dithering,
            color_precision,
            render_scale,
            audiovisual_effect,
            bloom_intensity,
            bloom_saturation,
            bloom_threshold,
            bloom_frequency_rotation,
            bloom_frequency_invert,
            invert_colors,
            flip_horizontal,
            flip_vertical,
            hue_rotation,
        ) = row;

        let target_defaults =
            match policy_target.as_str() {
                "screensaver" => Some(&screensaver_defaults),
                "wallpaper" => Some(&wallpaper_defaults),
                "unassigned" => None,
                other => {
                    return Err(
                        format!(
                            "Policy '{}' has unsupported target '{}'",
                            policy_name,
                            other,
                        )
                    );
                }
            };

        let texture = resolve_portable_texture(
            &policy_name,
            texture_mode.as_deref(),
            texture_family,
            texture_primitives,
            target_defaults,
        )?;

        let palette = resolve_portable_palette(
            &policy_name,
            palette_mode.as_deref(),
            palette_color,
            target_defaults,
        )?;

        let animation_speed =
            match animation_speed {
                Some(value) => PortableTargetValue::Explicit(value),
                None => {
                    match target_defaults {
                        Some(defaults) => {
                            PortableTargetValue::Explicit(
                                defaults.animation_speed
                            )
                        }
                        None => PortableTargetValue::InheritTarget,
                    }
                }
            };

        let bloom_frequency_invert =
            database_boolean(
                &policy_name,
                "bloom_frequency_invert",
                bloom_frequency_invert,
            )?;

        let invert_colors =
            database_boolean(
                &policy_name,
                "invert_colors",
                invert_colors,
            )?;

        let flip_horizontal =
            database_boolean(
                &policy_name,
                "flip_horizontal",
                flip_horizontal,
            )?;

        let flip_vertical =
            database_boolean(
                &policy_name,
                "flip_vertical",
                flip_vertical,
            )?;

        let portable_policy = PortablePolicy {
            policy_id,
            policy_name,
            policy_target,
            shader_id,
            shader_filename,
            shader_source_path,
            texture,
            palette,
            rendered_fps:
                rendered_fps.unwrap_or(app_defaults.rendered_fps),
            animation_speed,
            starting_offset,
            anti_aliasing:
                anti_aliasing.unwrap_or_else(
                    || app_defaults.anti_aliasing.clone()
                ),
            dithering:
                dithering.unwrap_or_else(
                    || app_defaults.dithering.clone()
                ),
            color_precision:
                color_precision.unwrap_or_else(
                    || app_defaults.color_precision.clone()
                ),
            render_scale:
                render_scale.unwrap_or(app_defaults.render_scale),
            audiovisual_effect,
            bloom_intensity,
            bloom_saturation,
            bloom_threshold,
            bloom_frequency_rotation,
            bloom_frequency_invert,
            invert_colors,
            flip_horizontal,
            flip_vertical,
            hue_rotation,
        };

        validate_portable_policy(
            &portable_policy
        )?;

        portable_policies.push(
            portable_policy
        );
    }

    if portable_policies.len() != selected_policy_ids.len() {
        return Err(
            "One or more selected policies disappeared while export data was being resolved"
                .to_string()
        );
    }

    Ok(portable_policies)
}

fn resolve_portable_texture(
    policy_name: &str,
    mode: Option<&str>,
    family: Option<String>,
    primitives: Option<i64>,
    target_defaults: Option<&crate::manage_configuration::TargetDefaults>,
) -> Result<PortableTextureSelection, String> {
    match mode {
        Some("specific") => {
            let family = family.ok_or_else(
                || {
                    format!(
                        "Policy '{}' has a specific texture without a texture family",
                        policy_name,
                    )
                }
            )?;

            let primitives = primitives.ok_or_else(
                || {
                    format!(
                        "Policy '{}' has a specific texture without a primitive count",
                        policy_name,
                    )
                }
            )?;

            Ok(
                PortableTextureSelection::Specific {
                    family,
                    primitives,
                }
            )
        }

        Some("random") => {
            Ok(
                PortableTextureSelection::Random {
                    primitives:
                        target_defaults.map(
                            |defaults| defaults.texture_primitives
                        ),
                }
            )
        }

        None => {
            match target_defaults {
                Some(defaults) => {
                    portable_texture_from_target_defaults(
                        policy_name,
                        defaults,
                    )
                }

                None => Ok(
                    PortableTextureSelection::InheritTarget
                ),
            }
        }

        Some(other) => Err(
            format!(
                "Policy '{}' has unsupported texture mode '{}'",
                policy_name,
                other,
            )
        ),
    }
}

fn portable_texture_from_target_defaults(
    policy_name: &str,
    defaults: &crate::manage_configuration::TargetDefaults,
) -> Result<PortableTextureSelection, String> {
    match defaults.texture_mode.as_str() {
        "specific" => {
            let family =
                defaults
                    .texture_family
                    .clone()
                    .ok_or_else(
                        || {
                            format!(
                                "{} defaults specify a specific texture without a texture family while resolving policy '{}'",
                                defaults.target,
                                policy_name,
                            )
                        }
                    )?;

            Ok(
                PortableTextureSelection::Specific {
                    family,
                    primitives: defaults.texture_primitives,
                }
            )
        }

        "random" => Ok(
            PortableTextureSelection::Random {
                primitives: Some(defaults.texture_primitives),
            }
        ),

        other => Err(
            format!(
                "{} defaults contain unsupported texture mode '{}' while resolving policy '{}'",
                defaults.target,
                other,
                policy_name,
            )
        ),
    }
}

fn resolve_portable_palette(
    policy_name: &str,
    mode: Option<&str>,
    color: Option<String>,
    target_defaults: Option<&crate::manage_configuration::TargetDefaults>,
) -> Result<PortablePaletteSelection, String> {
    match mode {
        Some("specific") => {
            let color = color.ok_or_else(
                || {
                    format!(
                        "Policy '{}' has a specific palette without a palette color",
                        policy_name,
                    )
                }
            )?;

            Ok(
                PortablePaletteSelection::Specific(color)
            )
        }

        Some("random") => Ok(
            PortablePaletteSelection::Random
        ),

        None => {
            match target_defaults {
                Some(defaults) => {
                    portable_palette_from_target_defaults(
                        policy_name,
                        defaults,
                    )
                }

                None => Ok(
                    PortablePaletteSelection::InheritTarget
                ),
            }
        }

        Some(other) => Err(
            format!(
                "Policy '{}' has unsupported palette mode '{}'",
                policy_name,
                other,
            )
        ),
    }
}

fn portable_palette_from_target_defaults(
    policy_name: &str,
    defaults: &crate::manage_configuration::TargetDefaults,
) -> Result<PortablePaletteSelection, String> {
    match defaults.palette_mode.as_str() {
        "specific" => {
            let color =
                defaults
                    .palette_color
                    .clone()
                    .ok_or_else(
                        || {
                            format!(
                                "{} defaults specify a specific palette without a palette color while resolving policy '{}'",
                                defaults.target,
                                policy_name,
                            )
                        }
                    )?;

            Ok(
                PortablePaletteSelection::Specific(color)
            )
        }

        "random" => Ok(
            PortablePaletteSelection::Random
        ),

        other => Err(
            format!(
                "{} defaults contain unsupported palette mode '{}' while resolving policy '{}'",
                defaults.target,
                other,
                policy_name,
            )
        ),
    }
}

fn database_boolean(
    policy_name: &str,
    field_name: &str,
    value: i64,
) -> Result<bool, String> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(
            format!(
                "Policy '{}' has invalid boolean value {} for {}",
                policy_name,
                other,
                field_name,
            )
        ),
    }
}

fn validate_portable_policy(
    policy: &PortablePolicy,
) -> Result<(), String> {
    let assigned =
        policy.policy_target == "screensaver"
            || policy.policy_target == "wallpaper";

    if assigned {
        if matches!(
            &policy.texture,
            PortableTextureSelection::InheritTarget
        ) {
            return Err(
                format!(
                    "Assigned policy '{}' retained unresolved target texture inheritance",
                    policy.policy_name,
                )
            );
        }

        if matches!(
            &policy.palette,
            PortablePaletteSelection::InheritTarget
        ) {
            return Err(
                format!(
                    "Assigned policy '{}' retained unresolved target palette inheritance",
                    policy.policy_name,
                )
            );
        }

        if matches!(
            &policy.animation_speed,
            PortableTargetValue::InheritTarget
        ) {
            return Err(
                format!(
                    "Assigned policy '{}' retained unresolved target animation-speed inheritance",
                    policy.policy_name,
                )
            );
        }
    }

    match &policy.texture {
        PortableTextureSelection::Specific {
            family,
            primitives,
        } => {
            if family.trim().is_empty() || *primitives <= 0 {
                return Err(
                    format!(
                        "Policy '{}' resolved to an invalid specific texture",
                        policy.policy_name,
                    )
                );
            }
        }

        PortableTextureSelection::Random {
            primitives,
        } => {
            if assigned && primitives.is_none() {
                return Err(
                    format!(
                        "Assigned policy '{}' resolved to random texture without an explicit primitive count",
                        policy.policy_name,
                    )
                );
            }

            if let Some(primitives) = primitives {
                if *primitives <= 0 {
                    return Err(
                        format!(
                            "Policy '{}' resolved to an invalid random-texture primitive count",
                            policy.policy_name,
                        )
                    );
                }
            }
        }

        PortableTextureSelection::InheritTarget => {}
    }

    if policy.rendered_fps <= 0
        || !policy.starting_offset.is_finite()
        || policy.starting_offset < 0.0
        || !policy.render_scale.is_finite()
        || policy.render_scale <= 0.0
        || !policy.bloom_intensity.is_finite()
        || !policy.bloom_saturation.is_finite()
        || !policy.bloom_threshold.is_finite()
        || !policy.bloom_frequency_rotation.is_finite()
        || !policy.hue_rotation.is_finite()
    {
        return Err(
            format!(
                "Policy '{}' resolved to one or more invalid numeric export values",
                policy.policy_name,
            )
        );
    }

    if let PortableTargetValue::Explicit(speed) =
        &policy.animation_speed
    {
        if !speed.is_finite() {
            return Err(
                format!(
                    "Policy '{}' resolved to an invalid animation speed",
                    policy.policy_name,
                )
            );
        }
    }

    Ok(())
}

fn load_export_policy_choices(
) -> Result<Vec<ExportPolicyChoice>, String> {
    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open screenshaver.db for export selection: {}",
                        error,
                    )
                }
            )?;

    let mut statement =
        connection
            .prepare(
                "SELECT
                     p.policy_id,
                     p.policy_name,
                     p.policy_target,
                     s.shader_id,
                     s.filename,
                     s.source_path,
                     pl.playlist_id,
                     pl.playlist_name
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 LEFT JOIN playlist_members AS pm
                   ON pm.policy_id = p.policy_id
                 LEFT JOIN playlists AS pl
                   ON pl.playlist_id = pm.playlist_id
                 ORDER BY
                     p.policy_name COLLATE NOCASE,
                     p.policy_name,
                     p.policy_id,
                     pl.playlist_name COLLATE NOCASE,
                     pl.playlist_name,
                     pl.playlist_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare export policy selection query: {}",
                        error,
                    )
                }
            )?;

    let rows =
        statement
            .query_map(
                [],
                |row| {
                    Ok(
                        (
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, Option<i64>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query policies for export selection: {}",
                        error,
                    )
                }
            )?;

    let mut policies =
        std::collections::BTreeMap::<
            i64,
            ExportPolicyChoice,
        >::new();

    for row in rows {
        let (
            policy_id,
            policy_name,
            policy_target,
            shader_id,
            shader_filename,
            shader_source_path,
            playlist_id,
            playlist_name,
        ) =
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode export policy selection row: {}",
                        error,
                    )
                }
            )?;

        let policy =
            policies
                .entry(policy_id)
                .or_insert_with(
                    || ExportPolicyChoice {
                        policy_id,
                        policy_name,
                        policy_target,
                        shader_id,
                        shader_filename,
                        shader_source_path,
                        playlists: Vec::new(),
                    }
                );

        if let (Some(playlist_id), Some(playlist_name)) =
            (playlist_id, playlist_name)
        {
            policy.playlists.push(
                (
                    playlist_id,
                    playlist_name,
                )
            );
        }
    }

    let mut policies =
        policies
            .into_values()
            .collect::<Vec<_>>();

    policies.sort_by(
        |left, right| {
            left.policy_name
                .to_lowercase()
                .cmp(
                    &right.policy_name.to_lowercase()
                )
                .then_with(
                    || {
                        left.policy_name
                            .cmp(&right.policy_name)
                    }
                )
                .then_with(
                    || left.policy_id.cmp(&right.policy_id)
                )
        }
    );

    Ok(policies)
}

fn state_id() -> egui::Id {
    egui::Id::new("screenshaver_export_data_wizard_state")
}

pub fn open(ctx: &egui::Context) {
    let id = state_id();

    ctx.data_mut(
        |data| {
            let mut state = data
                .get_temp::<ExportWizardState>(id)
                .unwrap_or_default();

            state.reset_for_open();

            data.insert_temp(
                id,
                state,
            );
        }
    );
}

pub fn set_destination(
    ctx: &egui::Context,
    destination: &std::path::Path,
) {
    let id = state_id();

    ctx.data_mut(
        |data| {
            let mut state = data
                .get_temp::<ExportWizardState>(id)
                .unwrap_or_default();

            if state.open {
                state.destination =
                    destination
                        .to_string_lossy()
                        .into_owned();

                data.insert_temp(
                    id,
                    state,
                );
            }
        }
    );
}


pub fn draw(
    ctx: &egui::Context,
    destination_browse_requested: &mut Option<std::path::PathBuf>,
) {
    let id = state_id();

    let mut state = ctx.data(
        |data| {
            data.get_temp::<ExportWizardState>(id)
                .unwrap_or_default()
        }
    );

    if !state.open {
        return;
    }

    let export_resolution_scale =
        (ctx.screen_rect().height()
            / crate::editor_layout::EDIT_WINDOW_REFERENCE_DISPLAY_HEIGHT)
            .clamp(
                crate::editor_layout::EDIT_WINDOW_SCALE_MIN,
                crate::editor_layout::EDIT_WINDOW_SCALE_MAX,
            );

    let export_window_height =
        crate::editor_layout::EDIT_WINDOW_REFERENCE_HEIGHT_PIXELS
            * export_resolution_scale;

    egui::Window::new("Export Screenshaver Data")
        .id(egui::Id::new("screenshaver_export_data_wizard_window"))
        .collapsible(false)
        .resizable(true)
        .default_width(720.0)
        .default_height(export_window_height)
        .show(
            ctx,
            |ui| {
                // The Export wizard must establish its vertical extent through
                // its content, not by locking the outer egui Window's minimum
                // and maximum heights. Keeping min_height == max_height made
                // vertical resizing impossible while width remained resizable.
                //
                // Reserve approximately the same vertical extent as the Control
                // Center reference geometry. The title bar/frame are outside
                // this content Ui, so leave a small allowance for them.
                let export_content_height =
                    (export_window_height - 48.0)
                        .max(560.0);

                ui.set_min_height(export_content_height);

                // Divide the already-established Export content height between
                // the wizard body and the navigation/footer. Do not derive this
                // from ui.available_height(): in a resizable egui Window that
                // value can allow child allocation to drive the outer window
                // larger than the monitor.
                let wizard_body_height =
                    (export_content_height - 64.0)
                        .max(0.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(
                        ui.available_width(),
                        wizard_body_height,
                    ),
                    egui::Layout::left_to_right(
                        egui::Align::Min,
                    ),
                    |ui| {
                        draw_stage_rail(
                            ui,
                            &mut state,
                        );

                        ui.separator();

                        ui.add_space(12.0);

                        ui.vertical(
                            |ui| {
                                ui.set_min_width(430.0);

                                match state.stage {
                                    ExportStage::SelectFocus => {
                                        draw_select_focus_page(
                                            ui,
                                            &mut state,
                                        );
                                    }

                                    ExportStage::SelectData => {
                                        draw_select_data_page(
                                            ui,
                                            &mut state,
                                        );
                                    }

                                    ExportStage::Destination => {
                                        draw_destination_page(
                                            ui,
                                            &mut state,
                                            destination_browse_requested,
                                        );
                                    }

                                    ExportStage::Review => {
                                        draw_review_page(
                                            ui,
                                            &state,
                                        );
                                    }

                                    ExportStage::Results => {
                                        draw_results_page(
                                            ui,
                                            &state,
                                        );
                                    }
                                }
                            },
                        );
                    },
                );

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                draw_navigation(
                    ui,
                    &mut state,
                );
            },
        );

    ctx.data_mut(
        |data| {
            data.insert_temp(
                id,
                state,
            );
        }
    );
}

fn draw_stage_rail(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    ui.vertical(
        |ui| {
            ui.set_min_width(180.0);

            let focus_response = ui.add_enabled(
                state.stage_enabled(ExportStage::SelectFocus)
                    || state.stage == ExportStage::SelectFocus,
                egui::SelectableLabel::new(
                    state.stage == ExportStage::SelectFocus,
                    egui::RichText::new("Select Export Focus")
                        .strong(),
                ),
            )
            .on_hover_text(
                "Choose whether Policies, Shaders, or Playlists will be the selectable focus that drives this export."
            );

            if focus_response.clicked()
                && state.stage_enabled(ExportStage::SelectFocus)
            {
                state.stage = ExportStage::SelectFocus;
            }

            ui.add_space(6.0);

            let select_enabled =
                state.stage_enabled(ExportStage::SelectData)
                    || state.stage == ExportStage::SelectData;

            let primary_tab =
                state.export_focus.tab_order()[0];

            let primary_response = ui.add_enabled(
                select_enabled,
                egui::SelectableLabel::new(
                    state.stage == ExportStage::SelectData
                        && state.select_data_tab == primary_tab,
                    egui::RichText::new(
                        state.export_focus.select_label()
                    )
                    .strong(),
                ),
            );

            if primary_response.clicked()
                && state.stage_enabled(ExportStage::SelectData)
            {
                state.stage = ExportStage::SelectData;
                state.select_data_tab = primary_tab;
            }

            for tab in state.export_focus.tab_order().into_iter().skip(1) {
                let response = ui.horizontal(
                    |ui| {
                        ui.add_space(18.0);
                        ui.add_enabled(
                            select_enabled,
                            egui::SelectableLabel::new(
                                state.stage == ExportStage::SelectData
                                    && state.select_data_tab == tab,
                                tab.label(),
                            ),
                        )
                    },
                ).inner;

                if response.clicked()
                    && state.stage_enabled(ExportStage::SelectData)
                {
                    state.stage = ExportStage::SelectData;
                    state.select_data_tab = tab;
                }
            }

            ui.add_space(6.0);

            for stage in [
                ExportStage::Destination,
                ExportStage::Review,
                ExportStage::Results,
            ] {
                let enabled =
                    state.stage_enabled(stage)
                        || state.stage == stage;

                let response = ui.add_enabled(
                    enabled,
                    egui::SelectableLabel::new(
                        state.stage == stage,
                        egui::RichText::new(stage.label())
                            .strong(),
                    ),
                );

                let response = match stage {
                    ExportStage::Destination => response.on_hover_text(
                        "Choose the directory where the portable Screenshaver export archive will be created."
                    ),
                    ExportStage::Review => response.on_hover_text(
                        "Review the selected export focus, included policies, shaders and playlists, and destination before starting Export."
                    ),
                    ExportStage::Results => response,
                    ExportStage::SelectFocus | ExportStage::SelectData => response,
                };

                if response.clicked()
                    && state.stage_enabled(stage)
                {
                    state.stage = stage;
                }
            }
        },
    );
}

fn draw_select_focus_page(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    ui.heading("Select Export Focus");
    ui.add_space(12.0);

    ui.horizontal(
        |ui| {
            ui.label("Export Focus:");

            let mut selected_focus =
                state.export_focus;

            let response =
                egui::ComboBox::from_id_source(
                    "screenshaver_export_focus"
                )
                .selected_text(
                    selected_focus.label()
                )
                .show_ui(
                    ui,
                    |ui| {
                        for focus in ExportSelectionRoot::ALL {
                            ui.selectable_value(
                                &mut selected_focus,
                                focus,
                                focus.label(),
                            );
                        }
                    },
                )
                .response
                .on_hover_text(
                    "Select the type of Screenshaver data that will drive this export. Policies lets you choose policies and automatically includes their required shaders and related playlists. Shaders lets you choose shaders and automatically includes their associated policies and playlists. Playlists lets you choose playlists and automatically includes their member policies and required shaders. Automatically included items are read-only."
                );

            if response.changed()
                || selected_focus != state.export_focus
            {
                state.set_export_focus(
                    selected_focus
                );
            }
        },
    );
}

fn draw_select_data_page(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    ui.heading(
        state.export_focus.select_label()
    );
    ui.add_space(8.0);

    ui.horizontal(
        |ui| {
            for tab in state.export_focus.tab_order() {
                if ui.selectable_label(
                    state.select_data_tab == tab,
                    tab.label(),
                )
                .clicked()
                {
                    state.select_data_tab = tab;
                }
            }
        },
    );

    ui.separator();
    ui.add_space(6.0);

    if let Some(error) = &state.selection_error {
        ui.label(
            egui::RichText::new(error)
                .strong()
        );
        return;
    }

    match state.select_data_tab {
        SelectDataTab::Policies => draw_export_policies_tab(ui, state),
        SelectDataTab::Shaders => draw_export_shaders_tab(ui, state),
        SelectDataTab::Playlists => draw_export_playlists_tab(ui, state),
    }
}

fn draw_export_policies_tab(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    let editable =
        state.export_focus == ExportSelectionRoot::Policies;

    if editable {
        ui.horizontal(
            |ui| {
                ui.strong("Policies");

                ui.label(
                    format!(
                        "{} of {} selected",
                        state.selected_policy_ids.len(),
                        state.policies.len(),
                    )
                );

                if ui.button("Select All").clicked() {
                    state.selected_policy_ids =
                        state.policies
                            .iter()
                            .map(
                                |policy| policy.policy_id
                            )
                            .collect();
                    state.refresh_portable_policies();
                }

                if ui.button("Clear All").clicked() {
                    state.selected_policy_ids.clear();
                    state.refresh_portable_policies();
                }
            },
        );
    } else {
        ui.strong(
            format!(
                "Policies Included ({})",
                state.included_policy_ids().len(),
            )
        );
    }

    ui.add_space(4.0);

    let included_policy_ids =
        state.included_policy_ids();

    let mut selection_changed = false;

    let list_height =
        ui.available_height()
            .max(0.0);

    egui::ScrollArea::vertical()
        .id_source("screenshaver_export_policy_selection")
        .max_height(list_height)
        .min_scrolled_height(list_height)
        .show(
            ui,
            |ui| {
                for policy in &state.policies {
                    if editable {
                        let mut selected =
                            state.selected_policy_ids.contains(
                                &policy.policy_id
                            );

                        let response =
                            ui.checkbox(
                                &mut selected,
                                format!(
                                    "{}  ({})",
                                    policy.policy_name,
                                    policy.policy_target,
                                ),
                            )
                            .on_hover_text(
                                std::path::Path::new(
                                    &policy.shader_source_path
                                )
                                .join(
                                    &policy.shader_filename
                                )
                                .display()
                                .to_string()
                            );

                        if response.changed() {
                            if selected {
                                state.selected_policy_ids.insert(
                                    policy.policy_id
                                );
                            } else {
                                state.selected_policy_ids.remove(
                                    &policy.policy_id
                                );
                            }

                            selection_changed = true;
                        }
                    } else if included_policy_ids.contains(
                        &policy.policy_id
                    ) {
                        let mut included = true;

                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(
                                &mut included,
                                format!(
                                    "{}  ({})",
                                    policy.policy_name,
                                    policy.policy_target,
                                ),
                            ),
                        )
                        .on_hover_text(
                            std::path::Path::new(
                                &policy.shader_source_path
                            )
                            .join(
                                &policy.shader_filename
                            )
                            .display()
                            .to_string()
                        );
                    }
                }
            },
        );

    if selection_changed {
        state.refresh_portable_policies();
    }

    draw_selection_status(ui, state, editable);
}

fn draw_export_shaders_tab(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    let editable =
        state.export_focus == ExportSelectionRoot::Shaders;

    if editable {
        ui.horizontal(
            |ui| {
                ui.strong("Shaders");

                ui.label(
                    format!(
                        "{} of {} selected",
                        state.selected_shader_ids.len(),
                        state.shaders.len(),
                    )
                );

                if ui.button("Select All").clicked() {
                    state.selected_shader_ids =
                        state.shaders
                            .iter()
                            .map(
                                |shader| shader.shader_id
                            )
                            .collect();
                    state.refresh_portable_policies();
                }

                if ui.button("Clear All").clicked() {
                    state.selected_shader_ids.clear();
                    state.refresh_portable_policies();
                }
            },
        );
    } else {
        ui.strong(
            format!(
                "Shaders Included ({})",
                state.included_shaders().len(),
            )
        );
    }

    ui.add_space(4.0);

    let included_shader_ids =
        state.included_shaders()
            .into_iter()
            .map(|(shader_id, _, _)| shader_id)
            .collect::<std::collections::HashSet<_>>();

    let mut selection_changed = false;

    let list_height =
        ui.available_height()
            .max(0.0);

    egui::ScrollArea::vertical()
        .id_source("screenshaver_export_shader_dependencies")
        .max_height(list_height)
        .min_scrolled_height(list_height)
        .show(
            ui,
            |ui| {
                for shader in &state.shaders {
                    if editable {
                        let mut selected =
                            state.selected_shader_ids.contains(
                                &shader.shader_id
                            );

                        let response =
                            ui.checkbox(
                                &mut selected,
                                &shader.filename,
                            )
                            .on_hover_text(
                                std::path::Path::new(
                                    &shader.source_path
                                )
                                .join(
                                    &shader.filename
                                )
                                .display()
                                .to_string()
                            );

                        if response.changed() {
                            if selected {
                                state.selected_shader_ids.insert(
                                    shader.shader_id
                                );
                            } else {
                                state.selected_shader_ids.remove(
                                    &shader.shader_id
                                );
                            }

                            selection_changed = true;
                        }
                    } else if included_shader_ids.contains(
                        &shader.shader_id
                    ) {
                        let mut included = true;

                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(
                                &mut included,
                                &shader.filename,
                            ),
                        )
                        .on_hover_text(
                            std::path::Path::new(
                                &shader.source_path
                            )
                            .join(
                                &shader.filename
                            )
                            .display()
                            .to_string()
                        );
                    }
                }
            },
        );

    if selection_changed {
        state.refresh_portable_policies();
    }

    draw_selection_status(ui, state, editable);
}

fn draw_export_playlists_tab(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    let editable =
        state.export_focus == ExportSelectionRoot::Playlists;

    if editable {
        ui.horizontal(
            |ui| {
                ui.strong("Playlists");

                ui.label(
                    format!(
                        "{} of {} selected",
                        state.selected_playlist_ids.len(),
                        state.playlists.len(),
                    )
                );

                if ui.button("Select All").clicked() {
                    state.selected_playlist_ids =
                        state.playlists
                            .iter()
                            .map(
                                |playlist| playlist.playlist_id
                            )
                            .collect();
                    state.refresh_portable_policies();
                }

                if ui.button("Clear All").clicked() {
                    state.selected_playlist_ids.clear();
                    state.refresh_portable_policies();
                }
            },
        );
    } else {
        ui.strong(
            format!(
                "Playlists Included ({})",
                state.included_playlists().len(),
            )
        );
    }

    ui.add_space(4.0);

    let included_playlist_ids =
        state.included_playlists()
            .into_iter()
            .map(|(playlist_id, _)| playlist_id)
            .collect::<std::collections::HashSet<_>>();

    let mut selection_changed = false;

    let list_height =
        ui.available_height()
            .max(0.0);

    egui::ScrollArea::vertical()
        .id_source("screenshaver_export_playlist_dependencies")
        .max_height(list_height)
        .min_scrolled_height(list_height)
        .show(
            ui,
            |ui| {
                for playlist in &state.playlists {
                    if editable {
                        let mut selected =
                            state.selected_playlist_ids.contains(
                                &playlist.playlist_id
                            );

                        let response =
                            ui.checkbox(
                                &mut selected,
                                &playlist.playlist_name,
                            );

                        if response.changed() {
                            if selected {
                                state.selected_playlist_ids.insert(
                                    playlist.playlist_id
                                );
                            } else {
                                state.selected_playlist_ids.remove(
                                    &playlist.playlist_id
                                );
                            }

                            selection_changed = true;
                        }
                    } else if included_playlist_ids.contains(
                        &playlist.playlist_id
                    ) {
                        let mut included = true;

                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(
                                &mut included,
                                &playlist.playlist_name,
                            ),
                        );
                    }
                }
            },
        );

    if selection_changed {
        state.refresh_portable_policies();
    }

    draw_selection_status(ui, state, editable);
}

fn draw_selection_status(
    ui: &mut egui::Ui,
    state: &ExportWizardState,
    editable: bool,
) {
    if let Some(error) = &state.portable_policy_error {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                format!(
                    "Unable to resolve effective export policies: {}",
                    error,
                )
            )
            .strong(),
        );
    } else if !state.included_policy_ids().is_empty() {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                format!(
                    "{} included policies resolved and validated for portable export.",
                    state.portable_policies.len(),
                )
            )
            .weak(),
        );
    }

    if editable && !state.focus_selection_nonempty() {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                format!(
                    "Select at least one {} to continue.",
                    state.export_focus.label(),
                )
            )
            .weak(),
        );
    }
}

fn draw_destination_page(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
    destination_browse_requested: &mut Option<std::path::PathBuf>,
) {
    ui.heading("Destination");
    ui.add_space(8.0);

    ui.label("Destination Folder:");

    ui.horizontal(
        |ui| {
            ui.add(
                egui::TextEdit::singleline(
                    &mut state.destination
                )
                .desired_width(360.0)
                .hint_text("$HOME"),
            );

            if ui.button("Browse...").clicked() {
                *destination_browse_requested =
                    Some(
                        std::path::PathBuf::from(
                            state.destination.trim()
                        )
                    );
            }
        },
    );

    ui.add_space(10.0);

    ui.label("Export Filename:");

    ui.add(
        egui::TextEdit::singleline(
            &mut state.export_filename
        )
        .desired_width(360.0)
        .hint_text(
            "Screenshaver-Export-YYYY-MM-DD-HHMMSS.zip"
        ),
    );

    if !valid_export_filename(
        &state.export_filename
    ) {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Enter a filename without directory separators."
            )
            .weak(),
        );
    } else if let Some(path) =
        state.resolved_export_path()
    {
        let requested =
            std::path::Path::new(
                state.destination.trim()
            )
            .join(
                state.export_filename.trim()
            );

        if path != requested {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    format!(
                        "That filename already exists. Export will use: {}",
                        path.file_name()
                            .and_then(
                                |value| value.to_str()
                            )
                            .unwrap_or(
                                state.export_filename.trim()
                            ),
                    )
                )
                .weak(),
            );
        }
    }
}

fn draw_review_page(
    ui: &mut egui::Ui,
    state: &ExportWizardState,
) {
    ui.heading("Review & Confirm");
    ui.add_space(12.0);

    ui.strong("Export Focus");
    ui.add_space(4.0);
    ui.label(state.export_focus.label());

    ui.add_space(14.0);

    ui.strong("Export Contents");
    ui.add_space(4.0);

    egui::Grid::new("export_review_contents")
        .num_columns(2)
        .spacing([24.0, 4.0])
        .show(
            ui,
            |ui| {
                ui.label("Policies:");
                ui.label(
                    state.included_policy_ids()
                        .len()
                        .to_string()
                );
                ui.end_row();

                ui.label("Shaders:");
                ui.label(
                    state.included_shaders()
                        .len()
                        .to_string()
                );
                ui.end_row();

                ui.label("Playlists:");
                ui.label(
                    state.included_playlists()
                        .len()
                        .to_string()
                );
                ui.end_row();
            },
        );

    ui.add_space(14.0);

    ui.strong("Destination");
    ui.add_space(4.0);

    if let Some(path) =
        state.resolved_export_path()
    {
        ui.label(path.display().to_string());
    } else {
        ui.label(
            format!(
                "{} / {}",
                state.destination.trim(),
                state.export_filename.trim(),
            )
        );
    }

    ui.add_space(14.0);

    ui.strong("Export Format");
    ui.add_space(4.0);
    ui.label("Screenshaver Export Format 1");

    ui.add_space(18.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new(
            "No Screenshaver configuration will be changed. Export creates a portable copy of the items shown above."
        )
        .strong(),
    );
}

fn draw_results_page(
    ui: &mut egui::Ui,
    state: &ExportWizardState,
) {
    ui.heading("Results");
    ui.add_space(12.0);

    match state.export_result.as_ref() {
        Some(Ok(result)) => {
            ui.strong("Export completed successfully.");
            ui.add_space(8.0);
            ui.label(
                format!(
                    "Archive: {}",
                    result.path.display(),
                )
            );
            ui.add_space(8.0);
            ui.label(
                format!(
                    "Policies: {}    Shaders: {}    Playlists: {}",
                    result.policy_count,
                    result.shader_count,
                    result.playlist_count,
                )
            );
        }

        Some(Err(error)) => {
            ui.strong("Export failed.");
            ui.add_space(8.0);
            ui.label(error);
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "No completed export archive was installed."
                )
                .weak(),
            );
        }

        None => {
            ui.label("Export has not been run.");
        }
    }
}

fn draw_navigation(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    if state.stage == ExportStage::Review {
        ui.horizontal(
            |ui| {
                if ui.add_enabled(
                    !state.execution_started,
                    egui::Button::new("< Back"),
                )
                .clicked()
                {
                    state.stage =
                        ExportStage::Destination;
                }

                ui.with_layout(
                    egui::Layout::right_to_left(
                        egui::Align::Center
                    ),
                    |ui| {
                        if ui.add_enabled(
                            !state.execution_started
                                && state.select_data_valid()
                                && state.destination_valid(),
                            egui::Button::new("Export"),
                        )
                        .clicked()
                        {
                            state.execution_started = true;

                            let result =
                                create_export_archive(state);

                            state.export_result =
                                Some(result);

                            state.stage =
                                ExportStage::Results;
                        }

                        if ui.add_enabled(
                            !state.execution_started,
                            egui::Button::new("Cancel"),
                        )
                        .clicked()
                        {
                            state.open = false;
                        }
                    },
                );
            },
        );

        return;
    }

    ui.horizontal(
        |ui| {
            let back_enabled =
                !state.execution_started
                    && state.stage != ExportStage::SelectFocus;

            if ui.add_enabled(
                back_enabled,
                egui::Button::new("< Back"),
            )
            .clicked()
            {
                match state.stage {
                    ExportStage::SelectFocus => {}

                    ExportStage::SelectData => {
                        let order =
                            state.export_focus.tab_order();

                        let current_index =
                            order.iter()
                                .position(
                                    |tab| *tab == state.select_data_tab
                                )
                                .unwrap_or(0);

                        if current_index == 0 {
                            state.stage = ExportStage::SelectFocus;
                        } else {
                            state.select_data_tab =
                                order[current_index - 1];
                        }
                    }

                    ExportStage::Destination => {
                        state.stage = ExportStage::SelectData;
                        state.select_data_tab =
                            state.export_focus.tab_order()[2];
                    }

                    ExportStage::Review => {}

                    ExportStage::Results => {}
                }
            }

            let cancel_enabled = !state.execution_started;

            if ui.add_enabled(
                cancel_enabled,
                egui::Button::new("Cancel"),
            )
            .clicked()
            {
                state.open = false;
            }

            ui.with_layout(
                egui::Layout::right_to_left(
                    egui::Align::Center
                ),
                |ui| {
                    match state.stage {
                        ExportStage::SelectFocus => {
                            if ui.button("Next >").clicked() {
                                state.stage = ExportStage::SelectData;
                                state.select_data_tab =
                                    state.export_focus.tab_order()[0];
                            }
                        }

                        ExportStage::SelectData => {
                            let next_enabled =
                                state.select_data_valid();

                            if ui.add_enabled(
                                next_enabled,
                                egui::Button::new("Next >"),
                            )
                            .clicked()
                            {
                                let order =
                                    state.export_focus.tab_order();

                                let current_index =
                                    order.iter()
                                        .position(
                                            |tab| {
                                                *tab
                                                    == state.select_data_tab
                                            }
                                        )
                                        .unwrap_or(0);

                                if current_index < 2 {
                                    state.select_data_tab =
                                        order[current_index + 1];
                                } else {
                                    state.stage =
                                        ExportStage::Destination;
                                }
                            }
                        }

                        ExportStage::Destination => {
                            if ui.add_enabled(
                                state.select_data_valid()
                                    && state.destination_valid(),
                                egui::Button::new("Next >"),
                            )
                            .clicked()
                            {
                                state.stage = ExportStage::Review;
                            }
                        }

                        ExportStage::Review => {}

                        ExportStage::Results => {
                            if ui.button("Finish").clicked() {
                                state.open = false;
                            }
                        }
                    }
                },
            );
        },
    );
}
