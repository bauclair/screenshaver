// Export Data wizard UI.
//
// This module owns the transient Control Center workflow for exporting portable
// Screenshaver data. Policy Name is the sole selectable export unit. Required
// shader files and applicable playlist relationships are derived read-only from
// the selected policies. This checkpoint also resolves each selected policy into an in-memory portable
// effective configuration. Archive creation remains intentionally unimplemented.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExportStage {
    SelectData,
    Destination,
    Review,
    Results,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectDataTab {
    Policies,
    Shaders,
    Playlists,
}

impl SelectDataTab {
    const ALL: [SelectDataTab; 3] = [
        SelectDataTab::Policies,
        SelectDataTab::Shaders,
        SelectDataTab::Playlists,
    ];

    fn label(self) -> &'static str {
        match self {
            SelectDataTab::Policies => "Policies",
            SelectDataTab::Shaders => "Shaders",
            SelectDataTab::Playlists => "Playlists",
        }
    }
}

impl ExportStage {
    const ALL: [ExportStage; 4] = [
        ExportStage::SelectData,
        ExportStage::Destination,
        ExportStage::Review,
        ExportStage::Results,
    ];

    fn label(self) -> &'static str {
        match self {
            ExportStage::SelectData => "Select Policies",
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
    select_data_tab: SelectDataTab,
    policies: Vec<ExportPolicyChoice>,
    selected_policy_ids: std::collections::HashSet<i64>,
    selection_error: Option<String>,
    portable_policies: Vec<PortablePolicy>,
    portable_policy_error: Option<String>,
    destination: String,
    execution_started: bool,
}

impl Default for ExportWizardState {
    fn default() -> Self {
        Self {
            open: false,
            stage: ExportStage::SelectData,
            select_data_tab: SelectDataTab::Policies,
            policies: Vec::new(),
            selected_policy_ids: std::collections::HashSet::new(),
            selection_error: None,
            portable_policies: Vec::new(),
            portable_policy_error: None,
            destination: String::new(),
            execution_started: false,
        }
    }
}

impl ExportWizardState {
    fn select_data_valid(&self) -> bool {
        self.selection_error.is_none()
            && self.portable_policy_error.is_none()
            && !self.selected_policy_ids.is_empty()
    }

    fn destination_valid(&self) -> bool {
        !self.destination.trim().is_empty()
    }

    fn stage_enabled(&self, stage: ExportStage) -> bool {
        match stage {
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

        let selected_policy_ids =
            policies
                .iter()
                .map(|policy| policy.policy_id)
                .collect();

        *self = Self {
            open: true,
            stage: ExportStage::SelectData,
            select_data_tab: SelectDataTab::Policies,
            policies,
            selected_policy_ids,
            selection_error,
            portable_policies: Vec::new(),
            portable_policy_error: None,
            destination: String::new(),
            execution_started: false,
        };

        self.refresh_portable_policies();
    }

    fn refresh_portable_policies(&mut self) {
        if self.selection_error.is_some()
            || self.selected_policy_ids.is_empty()
        {
            self.portable_policies.clear();
            self.portable_policy_error = None;
            return;
        }

        match resolve_portable_policies(
            &self.selected_policy_ids
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

    fn selected_policy_count(&self) -> usize {
        self.selected_policy_ids.len()
    }

    fn included_shaders(&self) -> Vec<(i64, String, String)> {
        let mut shaders =
            std::collections::BTreeMap::<
                i64,
                (String, String),
            >::new();

        for policy in &self.policies {
            if self.selected_policy_ids.contains(
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
        let mut playlists =
            std::collections::BTreeMap::<i64, String>::new();

        for policy in &self.policies {
            if self.selected_policy_ids.contains(
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
        .min_height(export_window_height)
        .max_height(export_window_height)
        .show(
            ctx,
            |ui| {
                ui.horizontal(
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

            let select_enabled =
                state.stage_enabled(ExportStage::SelectData)
                    || state.stage == ExportStage::SelectData;

            let select_response = ui.add_enabled(
                select_enabled,
                egui::SelectableLabel::new(
                    state.stage == ExportStage::SelectData,
                    egui::RichText::new("Select Policies")
                        .strong(),
                ),
            )
            .on_hover_text(
                "Choose the Policy Names to export. Required shaders and applicable playlist relationships are derived automatically."
            );

            if select_response.clicked()
                && state.stage_enabled(ExportStage::SelectData)
            {
                state.stage = ExportStage::SelectData;
                state.select_data_tab = SelectDataTab::Policies;
            }

            for tab in [SelectDataTab::Shaders, SelectDataTab::Playlists] {
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
                        "Review the selected policies, derived shaders and playlists, and destination before starting Export."
                    ),
                    ExportStage::Results => response,
                    ExportStage::SelectData => response,
                };

                if response.clicked()
                    && state.stage_enabled(stage)
                {
                    state.stage = stage;
                }

                ui.add_space(6.0);
            }
        },
    );
}

fn draw_select_data_page(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    ui.heading("Select Policies");
    ui.add_space(8.0);

    ui.horizontal(
        |ui| {
            for tab in SelectDataTab::ALL {
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
    ui.horizontal(
        |ui| {
            ui.strong("Policies");

            ui.label(
                format!(
                    "{} of {} selected",
                    state.selected_policy_count(),
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

    ui.add_space(4.0);

    let mut policy_selection_changed = false;

    egui::ScrollArea::vertical()
        .id_source("screenshaver_export_policy_selection")
        .max_height(430.0)
        .show(
            ui,
            |ui| {
                for policy in &state.policies {
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

                        policy_selection_changed = true;
                    }
                }
            },
        );

    if policy_selection_changed {
        state.refresh_portable_policies();
    }

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
    } else if !state.selected_policy_ids.is_empty() {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                format!(
                    "{} selected policies resolved and validated for portable export.",
                    state.portable_policies.len(),
                )
            )
            .weak(),
        );
    }

    if !state.select_data_valid() {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Select at least one Policy Name to continue."
            )
            .weak(),
        );
    }
}

fn draw_export_shaders_tab(
    ui: &mut egui::Ui,
    state: &ExportWizardState,
) {
    let included_shaders = state.included_shaders();

    ui.strong(
        format!(
            "Shaders Included ({})",
            included_shaders.len(),
        )
    );
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_source("screenshaver_export_shader_dependencies")
        .max_height(430.0)
        .show(
            ui,
            |ui| {
                for (
                    _shader_id,
                    filename,
                    source_path,
                ) in included_shaders
                {
                    let mut included = true;

                    ui.add_enabled(
                        false,
                        egui::Checkbox::new(
                            &mut included,
                            &filename,
                        ),
                    )
                    .on_hover_text(
                        std::path::Path::new(
                            &source_path
                        )
                        .join(
                            &filename
                        )
                        .display()
                        .to_string()
                    );
                }
            },
        );
}

fn draw_export_playlists_tab(
    ui: &mut egui::Ui,
    state: &ExportWizardState,
) {
    let included_playlists = state.included_playlists();

    ui.strong(
        format!(
            "Playlists Included ({})",
            included_playlists.len(),
        )
    );
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_source("screenshaver_export_playlist_dependencies")
        .max_height(430.0)
        .show(
            ui,
            |ui| {
                for (
                    _playlist_id,
                    playlist_name,
                ) in included_playlists
                {
                    let mut included = true;

                    ui.add_enabled(
                        false,
                        egui::Checkbox::new(
                            &mut included,
                            playlist_name,
                        ),
                    );
                }
            },
        );
}

fn draw_destination_page(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
    destination_browse_requested: &mut Option<std::path::PathBuf>,
) {
    ui.heading("Destination");
    ui.add_space(8.0);

    ui.label("Export destination:");

    ui.horizontal(
        |ui| {
            ui.add(
                egui::TextEdit::singleline(
                    &mut state.destination
                )
                .desired_width(360.0)
                .hint_text("/path/to/export/destination"),
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

}

fn draw_review_page(
    ui: &mut egui::Ui,
    state: &ExportWizardState,
) {
    ui.heading("Review & Confirm");
    ui.add_space(8.0);

    ui.label(
        format!(
            "Policies: {} selected",
            state.selected_policy_count(),
        )
    );

    ui.label(
        format!(
            "Effective policies: {} resolved and validated",
            state.portable_policies.len(),
        )
    );

    ui.label(
        format!(
            "Shaders: {} included automatically",
            state.included_shaders().len(),
        )
    );

    ui.label(
        format!(
            "Playlists: {} included automatically",
            state.included_playlists().len(),
        )
    );

    ui.add_space(8.0);

    ui.label(
        format!(
            "Destination: {}",
            state.destination.trim(),
        )
    );

}

fn draw_results_page(
    ui: &mut egui::Ui,
) {
    ui.heading("Results");
    ui.add_space(8.0);

    ui.label(
        "Effective export policy checkpoint completed successfully."
    );

    ui.add_space(8.0);

    ui.label(
        egui::RichText::new(
            "No files were written. Archive creation is not implemented in this checkpoint."
        )
        .weak(),
    );
}

fn draw_navigation(
    ui: &mut egui::Ui,
    state: &mut ExportWizardState,
) {
    ui.horizontal(
        |ui| {
            let back_enabled =
                !state.execution_started
                    && !(
                        state.stage == ExportStage::SelectData
                            && state.select_data_tab == SelectDataTab::Policies
                    );

            if ui.add_enabled(
                back_enabled,
                egui::Button::new("< Back"),
            )
            .clicked()
            {
                match state.stage {
                    ExportStage::SelectData => {
                        state.select_data_tab = match state.select_data_tab {
                            SelectDataTab::Policies => SelectDataTab::Policies,
                            SelectDataTab::Shaders => SelectDataTab::Policies,
                            SelectDataTab::Playlists => SelectDataTab::Shaders,
                        };
                    }

                    ExportStage::Destination => {
                        state.stage = ExportStage::SelectData;
                        state.select_data_tab = SelectDataTab::Playlists;
                    }

                    ExportStage::Review => {
                        state.stage = ExportStage::Destination;
                    }

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
                        ExportStage::SelectData => {
                            let next_enabled = match state.select_data_tab {
                                SelectDataTab::Policies => state.select_data_valid(),
                                SelectDataTab::Shaders => state.select_data_valid(),
                                SelectDataTab::Playlists => state.select_data_valid(),
                            };

                            if ui.add_enabled(
                                next_enabled,
                                egui::Button::new("Next >"),
                            )
                            .clicked()
                            {
                                match state.select_data_tab {
                                    SelectDataTab::Policies => {
                                        state.select_data_tab = SelectDataTab::Shaders;
                                    }
                                    SelectDataTab::Shaders => {
                                        state.select_data_tab = SelectDataTab::Playlists;
                                    }
                                    SelectDataTab::Playlists => {
                                        state.stage = ExportStage::Destination;
                                    }
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

                        ExportStage::Review => {
                            if ui.add_enabled(
                                state.select_data_valid()
                                    && state.destination_valid(),
                                egui::Button::new("Export"),
                            )
                            .clicked()
                            {
                                // Effective-policy checkpoint only. This marks the
                                // execution boundary and unlocks Results without
                                // performing any persistent operation.
                                state.execution_started = true;
                                state.stage = ExportStage::Results;
                            }
                        }

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
