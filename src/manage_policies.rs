use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use toml_edit::{
    value,
    DocumentMut,
    Item,
    Table,
};


const POLICY_ID_SEPARATOR: &str = "::screenshaver-path::";


//
// ------------------------------------------------------------
// Public policy structures
// ------------------------------------------------------------
//

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum PolicyTarget {

    Screensaver,

    Wallpaper,

    Unassigned,
}


impl PolicyTarget {

    pub fn parse(
        value: &str,
    ) -> Result<Self, String> {

        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "screensaver" => {
                Ok(
                    Self::Screensaver
                )
            }

            "wallpaper" => {
                Ok(
                    Self::Wallpaper
                )
            }

            "unassigned" => {
                Ok(
                    Self::Unassigned
                )
            }

            other => {
                Err(
                    format!(
                        "Unknown policy target '{}'; supported targets: screensaver, wallpaper, unassigned",
                        other,
                    )
                )
            }
        }
    }


    pub fn name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver"
            }

            Self::Wallpaper => {
                "wallpaper"
            }

            Self::Unassigned => {
                "unassigned"
            }
        }
    }


    pub fn table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver_policies"
            }

            Self::Wallpaper => {
                "wallpaper_policies"
            }

            Self::Unassigned => {
                "unassigned_policies"
            }
        }
    }


    pub fn path_table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver_external_paths"
            }

            Self::Wallpaper => {
                "wallpaper_external_paths"
            }

            Self::Unassigned => {
                "unassigned_external_paths"
            }
        }
    }


    fn legacy_path_table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver_shader_paths"
            }

            Self::Wallpaper => {
                "wallpaper_shader_paths"
            }

            Self::Unassigned => {
                "unassigned_shader_paths"
            }
        }
    }
}


#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
)]
pub struct PolicyDefinition {

    pub texture:
        Option<String>,

    pub palette:
        Option<String>,

    pub fps:
        Option<u32>,

    pub speed:
        Option<f32>,

    pub render_scale:
        Option<f32>,

    pub anti_aliasing:
        Option<String>,

    pub dithering:
        Option<String>,

    pub color_precision:
        Option<String>,

    pub bloom:
        Option<String>,

    pub bloom_intensity:
        Option<f32>,

    pub bloom_threshold:
        Option<f32>,

    pub invert_colors:
        Option<bool>,

    pub flip_horizontal:
        Option<bool>,

    pub flip_vertical:
        Option<bool>,

    pub hue_rotation:
        Option<f32>,
}


#[derive(
    Debug,
    Clone,
)]
pub struct BulkPolicyCreation {

    pub target:
        PolicyTarget,

    pub shader:
        String,

    pub source_path:
        PathBuf,

    pub properties:
        PolicyDefinition,
}


#[derive(
    Debug,
    Clone,
    Copy,
    Default,
)]
pub struct BulkPolicyCreationResult {

    pub created:
        usize,

    pub skipped_existing:
        usize,
}


#[derive(
    Debug,
    Clone,
)]
pub struct BulkPolicyReplacement {

    pub target:
        PolicyTarget,

    pub policy_key:
        String,

    pub properties:
        PolicyDefinition,
}


impl PolicyDefinition {

    pub fn is_empty(
        &self,
    ) -> bool {

        self.texture.is_none()
            && self.palette.is_none()
            && self.fps.is_none()
            && self.speed.is_none()
            && self.render_scale.is_none()
            && self.anti_aliasing.is_none()
            && self.dithering.is_none()
            && self.color_precision.is_none()
            && self.bloom.is_none()
            && self.bloom_intensity.is_none()
            && self.bloom_threshold.is_none()
            && self.invert_colors.is_none()
            && self.flip_horizontal.is_none()
            && self.flip_vertical.is_none()
            && self.hue_rotation.is_none()
    }
}


//
// ------------------------------------------------------------
// Public configuration operations
// ------------------------------------------------------------
//

pub fn retarget_policy_by_key(
    policy_key: &str,
    destination_target: PolicyTarget,
) -> Result<(), String> {

    let policy_name =
        policy_key.trim();


    if policy_name.is_empty() {

        return Err(
            "Policy Name may not be empty"
                .to_string()
        );
    }


    let policy_name_key =
        database_policy_name_key(
            policy_name
        )?;


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while changing policy target for '{}': {}",
                        policy_name,
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin policy retarget transaction for '{}': {}",
                        policy_name,
                        error,
                    )
                }
            )?;


    let (
        policy_id,
        filename,
        current_target,
        stored_policy_name,
    ):
        (
            i64,
            String,
            String,
            String,
        ) =
        transaction
            .query_row(
                "SELECT
                     p.policy_id,
                     s.filename,
                     p.policy_target,
                     p.policy_name
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_name_key = ?1",
                rusqlite::params![
                    policy_name_key
                ],
                |row| {
                    Ok(
                        (
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to locate policy '{}' for retargeting: {}",
                        policy_name,
                        error,
                    )
                }
            )?;


    let generated_current_name =
        format!(
            "{} ({})",
            filename,
            current_target,
        );


    let (
        replacement_name,
        replacement_key,
    ) =
        if stored_policy_name.eq_ignore_ascii_case(
            &generated_current_name
        ) {
            let replacement_name =
                format!(
                    "{} ({})",
                    filename,
                    destination_target.name(),
                );

            let replacement_key =
                database_policy_name_key(
                    &replacement_name
                )?;

            (
                replacement_name,
                replacement_key,
            )
        } else {
            (
                stored_policy_name,
                database_policy_name_key(
                    policy_name
                )?,
            )
        };


    transaction
        .execute(
            "UPDATE shader_policies
             SET policy_target = ?1,
                 policy_name = ?2,
                 policy_name_key = ?3
             WHERE policy_id = ?4",
            rusqlite::params![
                destination_target.name(),
                replacement_name,
                replacement_key,
                policy_id,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to change Policy Target for '{}': {}",
                    policy_name,
                    error,
                )
            }
        )?;


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit Policy Target change for '{}': {}",
                    policy_name,
                    error,
                )
            }
        )
}

pub fn assign_unassigned_policies_by_key(
    policy_keys: &[String],
    destination_target: PolicyTarget,
) -> Result<usize, String> {

    if policy_keys.is_empty() {

        return Ok(
            0
        );
    }


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while assigning unassigned policies: {}",
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin unassigned-policy assignment transaction: {}",
                        error,
                    )
                }
            )?;


    let mut changed =
        0_usize;


    for policy_key in policy_keys {

        let policy_name =
            policy_key.trim();


        if policy_name.is_empty() {

            return Err(
                "Unassigned policy name may not be empty"
                    .to_string()
            );
        }


        let policy_name_key =
            database_policy_name_key(
                policy_name
            )?;


        let (
            policy_id,
            filename,
            stored_policy_name,
        ): (
            i64,
            String,
            String,
        ) =
            transaction
                .query_row(
                    "SELECT
                         p.policy_id,
                         s.filename,
                         p.policy_name
                     FROM shader_policies AS p
                     JOIN shaders AS s
                       ON s.shader_id = p.shader_id
                     WHERE p.policy_name_key = ?1
                       AND p.policy_target = 'unassigned'",
                    rusqlite::params![
                        policy_name_key
                    ],
                    |row| {
                        Ok(
                            (
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                            )
                        )
                    },
                )
                .map_err(
                    |error| {
                        format!(
                            "Policy '{}' is no longer an Unassigned policy: {}",
                            policy_name,
                            error,
                        )
                    }
                )?;


        let generated_unassigned_name =
            format!(
                "{} (unassigned)",
                filename,
            );


        if stored_policy_name
            .eq_ignore_ascii_case(
                &generated_unassigned_name
            )
        {
            let desired_name =
                format!(
                    "{} ({})",
                    filename,
                    destination_target.name(),
                );


            let desired_key =
                database_policy_name_key(
                    &desired_name
                )?;


            let name_conflict_count: i64 =
                transaction
                    .query_row(
                        "SELECT COUNT(*)
                         FROM shader_policies
                         WHERE policy_name_key = ?1
                           AND policy_id <> ?2",
                        rusqlite::params![
                            desired_key,
                            policy_id,
                        ],
                        |row| {
                            row.get(0)
                        },
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to validate generated Policy Name '{}': {}",
                                desired_name,
                                error,
                            )
                        }
                    )?;


            if name_conflict_count
                == 0
            {
                transaction
                    .execute(
                        "UPDATE shader_policies
                         SET policy_target = ?1,
                             policy_name = ?2,
                             policy_name_key = ?3
                         WHERE policy_id = ?4
                           AND policy_target = 'unassigned'",
                        rusqlite::params![
                            destination_target.name(),
                            desired_name,
                            desired_key,
                            policy_id,
                        ],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to assign and rename policy '{}': {}",
                                stored_policy_name,
                                error,
                            )
                        }
                    )?;
            } else {
                // Preserve the existing name rather than inventing a new user-
                // visible name when the canonical target name is already used.
                transaction
                    .execute(
                        "UPDATE shader_policies
                         SET policy_target = ?1
                         WHERE policy_id = ?2
                           AND policy_target = 'unassigned'",
                        rusqlite::params![
                            destination_target.name(),
                            policy_id,
                        ],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to assign policy '{}': {}",
                                stored_policy_name,
                                error,
                            )
                        }
                    )?;
            }
        } else {
            // A custom Policy Name belongs to the user and is never rewritten
            // merely because its target changes.
            transaction
                .execute(
                    "UPDATE shader_policies
                     SET policy_target = ?1
                     WHERE policy_id = ?2
                       AND policy_target = 'unassigned'",
                    rusqlite::params![
                        destination_target.name(),
                        policy_id,
                    ],
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to assign policy '{}': {}",
                            stored_policy_name,
                            error,
                        )
                    }
                )?;
        }


        changed +=
            1;
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit unassigned-policy assignment transaction: {}",
                    error,
                )
            }
        )?;


    Ok(
        changed
    )
}


pub fn policy_exists_for_source(
    _config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    source_path: &Path,
) -> Result<bool, String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    let shader_id =
        database_shader_id_for_source(
            &shader,
            source_path,
        )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while checking {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    let count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM shader_policies
                 WHERE shader_id = ?1
                   AND policy_target = ?2",
                rusqlite::params![
                    shader_id,
                    target.name(),
                ],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to check {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    Ok(
        count > 0
    )
}

pub fn add_policy_for_source(
    _config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
    source_path: &Path,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    validate_properties(
        target,
        &properties
    )?;


    let shader_id =
        database_shader_id_for_source(
            &shader,
            source_path,
        )?;


    let values =
        database_policy_values(
            &properties
        )?;


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while adding {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin database transaction while adding {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    let existing_count: i64 =
        transaction
            .query_row(
                "SELECT COUNT(*)
                 FROM shader_policies
                 WHERE shader_id = ?1
                   AND policy_target = ?2",
                rusqlite::params![
                    shader_id,
                    target.name(),
                ],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to check existing {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    if existing_count > 0 {

        return Err(
            format!(
                "Shader '{}' already has a {} policy",
                shader,
                target.name(),
            )
        );
    }


    let policy_name =
        next_database_policy_name(
            &transaction,
            &shader,
        )?;


    let policy_name_key =
        database_policy_name_key(
            &policy_name
        )?;


    transaction
        .execute(
            "INSERT INTO shader_policies (
                 policy_name,
                 policy_name_key,
                 shader_id,
                 policy_target,
                 texture_mode,
                 texture_family,
                 texture_primitives,
                 palette_mode,
                 palette_color,
                 rendered_fps,
                 animation_speed,
                 anti_aliasing,
                 dithering,
                 color_precision,
                 render_scale,
                 bloom_mode,
                 bloom_intensity,
                 bloom_threshold,
                 invert_colors,
                 flip_horizontal,
                 flip_vertical,
                 hue_rotation
             )
             VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                 ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                 ?21, ?22
             )",
            rusqlite::params![
                policy_name,
                policy_name_key,
                shader_id,
                target.name(),
                values.texture_mode,
                values.texture_family,
                values.texture_primitives,
                values.palette_mode,
                values.palette_color,
                values.rendered_fps,
                values.animation_speed,
                values.anti_aliasing,
                values.dithering,
                values.color_precision,
                values.render_scale,
                values.bloom_mode,
                values.bloom_intensity,
                values.bloom_threshold,
                values.invert_colors,
                values.flip_horizontal,
                values.flip_vertical,
                values.hue_rotation,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to insert {} policy for '{}': {}",
                    target.name(),
                    shader,
                    error,
                )
            }
        )?;


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit {} policy for '{}': {}",
                    target.name(),
                    shader,
                    error,
                )
            }
        )
}

pub fn replace_policy_for_source(
    _config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
    source_path: &Path,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    validate_properties(
        target,
        &properties
    )?;


    let shader_id =
        database_shader_id_for_source(
            &shader,
            source_path,
        )?;


    let values =
        database_policy_values(
            &properties
        )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while replacing {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    let policy_ids =
        {
            let mut statement =
                connection
                    .prepare(
                        "SELECT policy_id
                         FROM shader_policies
                         WHERE shader_id = ?1
                           AND policy_target = ?2
                         ORDER BY policy_id"
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to prepare {} policy lookup for '{}': {}",
                                target.name(),
                                shader,
                                error,
                            )
                        }
                    )?;


            let rows =
                statement
                    .query_map(
                        rusqlite::params![
                            shader_id,
                            target.name(),
                        ],
                        |row| {
                            row.get::<_, i64>(0)
                        },
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to query {} policy for '{}': {}",
                                target.name(),
                                shader,
                                error,
                            )
                        }
                    )?;


            let mut policy_ids =
                Vec::new();


            for row in rows {
                policy_ids.push(
                    row.map_err(
                        |error| {
                            format!(
                                "Unable to decode {} policy for '{}': {}",
                                target.name(),
                                shader,
                                error,
                            )
                        }
                    )?
                );
            }


            policy_ids
        };


    let policy_id =
        match policy_ids.as_slice() {

            [] => {
                return Err(
                    format!(
                        "Shader '{}' does not have a {} policy",
                        shader,
                        target.name(),
                    )
                );
            }

            [policy_id] => {
                *policy_id
            }

            _ => {
                return Err(
                    format!(
                        "Shader '{}' has multiple {} policies; source-based replacement is ambiguous",
                        shader,
                        target.name(),
                    )
                );
            }
        };


    let changed =
        connection
            .execute(
                "UPDATE shader_policies
                 SET texture_mode = ?1,
                     texture_family = ?2,
                     texture_primitives = ?3,
                     palette_mode = ?4,
                     palette_color = ?5,
                     rendered_fps = ?6,
                     animation_speed = ?7,
                     anti_aliasing = ?8,
                     dithering = ?9,
                     color_precision = ?10,
                     render_scale = ?11,
                     bloom_mode = ?12,
                     bloom_intensity = ?13,
                     bloom_threshold = ?14,
                     invert_colors = ?15,
                     flip_horizontal = ?16,
                     flip_vertical = ?17,
                     hue_rotation = ?18
                 WHERE policy_id = ?19",
                rusqlite::params![
                    values.texture_mode,
                    values.texture_family,
                    values.texture_primitives,
                    values.palette_mode,
                    values.palette_color,
                    values.rendered_fps,
                    values.animation_speed,
                    values.anti_aliasing,
                    values.dithering,
                    values.color_precision,
                    values.render_scale,
                    values.bloom_mode,
                    values.bloom_intensity,
                    values.bloom_threshold,
                    values.invert_colors,
                    values.flip_horizontal,
                    values.flip_vertical,
                    values.hue_rotation,
                    policy_id,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to update {} policy for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    if changed != 1 {

        return Err(
            format!(
                "Expected to update one {} policy for '{}', updated {}",
                target.name(),
                shader,
                changed,
            )
        );
    }


    Ok(())
}

pub fn add_policies_for_sources(
    config_path: &Path,
    creations: &[BulkPolicyCreation],
) -> Result<BulkPolicyCreationResult, String> {

    if creations.is_empty() {
        return Ok(
            BulkPolicyCreationResult::default()
        );
    }


    let mut document =
        load_document(
            config_path
        )?;


    let mut result =
        BulkPolicyCreationResult::default();


    for creation in creations {

        let shader =
            normalized_shader_name(
                &creation.shader
            )?;


        validate_properties(
            creation.target,
            &creation.properties,
        )?;


        let source_path =
            canonical_or_absolute(
                &creation.source_path
            )?;


        if matching_policy_key_for_source(
            &document,
            creation.target,
            &shader,
            &source_path,
        )?
        .is_some()
        {
            result.skipped_existing +=
                1;

            continue;
        }


        let storage_key =
            {
                let table =
                    policy_table_mut(
                        &mut document,
                        creation.target,
                    )?;


                unique_policy_storage_key(
                    table,
                    &shader,
                    &source_path,
                )
            };


        {
            let table =
                policy_table_mut(
                    &mut document,
                    creation.target,
                )?;


            table[
                &storage_key
            ] =
                value(
                    format_policy(
                        &creation.properties
                    )
                );
        }


        let managed_path =
            managed_shader_path(
                creation.target,
                &shader,
            );


        if paths_refer_to_same_source(
            &source_path,
            &managed_path,
        ) {
            remove_source_path_metadata(
                &mut document,
                creation.target,
                &storage_key,
            )?;
        } else {
            let source_text =
                source_path
                    .to_str()
                    .ok_or_else(
                        || {
                            format!(
                                "Shader path is not valid UTF-8: {}",
                                source_path.display(),
                            )
                        }
                    )?;


            set_source_path_metadata(
                &mut document,
                creation.target,
                &storage_key,
                Some(source_text),
            )?;
        }


        result.created +=
            1;
    }


    if result.created > 0 {
        save_document(
            config_path,
            &document,
        )?;
    }


    Ok(
        result
    )
}


pub fn replace_policies_by_key(
    config_path: &Path,
    replacements: &[BulkPolicyReplacement],
) -> Result<(), String> {

    if replacements.is_empty() {
        return Ok(());
    }


    let mut document =
        load_document(
            config_path
        )?;


    // Apply every requested update to the in-memory TOML document first.
    // save_document() is called only after every policy has been located and
    // validated, so a pre-write failure cannot leave a partially updated set.
    for replacement in replacements {

        validate_properties(
            replacement.target,
            &replacement.properties,
        )?;


        let table =
            policy_table_mut(
                &mut document,
                replacement.target,
            )?;


        let existing_key =
            table
                .iter()
                .find_map(
                    |(key, _)| {
                        if key.eq_ignore_ascii_case(
                            &replacement.policy_key
                        ) {
                            Some(
                                key.to_string()
                            )
                        } else {
                            None
                        }
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Policy key '{}' does not exist in [{}]",
                            replacement.policy_key,
                            replacement.target.table_name(),
                        )
                    }
                )?;


        table[
            &existing_key
        ] =
            value(
                format_policy(
                    &replacement.properties
                )
            );
    }


    // Intentionally leave source-path metadata untouched. Bulk Edit is
    // permitted to change policy properties only; it may not change policy
    // target assignments or shader source locations.
    save_document(
        config_path,
        &document,
    )
}


pub fn delete_policy_by_key(
    _config_path: &Path,
    target: PolicyTarget,
    policy_key: &str,
) -> Result<(), String> {

    let policy_name =
        policy_key.trim();


    if policy_name.is_empty() {

        return Err(
            "Policy key may not be empty"
                .to_string()
        );
    }


    let policy_name_key =
        database_policy_name_key(
            policy_name
        )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while deleting policy '{}': {}",
                        policy_name,
                        error,
                    )
                }
            )?;


    let deleted =
        connection
            .execute(
                "DELETE FROM shader_policies
                 WHERE policy_name_key = ?1
                   AND policy_target = ?2",
                rusqlite::params![
                    policy_name_key,
                    target.name(),
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to delete {} policy '{}': {}",
                        target.name(),
                        policy_name,
                        error,
                    )
                }
            )?;


    match deleted {

        1 => {
            Ok(())
        }

        0 => {
            Err(
                format!(
                    "Policy '{}' does not exist for target {}",
                    policy_name,
                    target.name(),
                )
            )
        }

        count => {
            Err(
                format!(
                    "Deleting policy '{}' unexpectedly removed {} rows",
                    policy_name,
                    count,
                )
            )
        }
    }
}

pub fn reconcile_shader_move_from_source(
    config_path: &Path,
    original_source_path: &Path,
    destination_path: &Path,
    destination_target: PolicyTarget,
) -> Result<(), String> {

    if !destination_path.is_absolute() {
        return Err(
            format!(
                "Moved shader path must be absolute: {}",
                destination_path.display(),
            )
        );
    }


    let destination_name =
        destination_path
            .file_name()
            .and_then(
                |name| name.to_str()
            )
            .ok_or_else(
                || {
                    format!(
                        "Moved shader filename is not valid UTF-8: {}",
                        destination_path.display(),
                    )
                }
            )?;


    let expected_destination =
        managed_shader_path(
            destination_target,
            destination_name,
        );


    if !paths_refer_to_same_source(
        destination_path,
        &expected_destination,
    ) {
        return Err(
            format!(
                "Destination '{}' is not the managed {} shader location for '{}'",
                destination_path.display(),
                destination_target.name(),
                destination_name,
            )
        );
    }


    let original_source_path =
        canonical_or_absolute(
            original_source_path
        )?;


    let mut document =
        load_document(
            config_path
        )?;


    let mut matching_policies:
        Vec<(PolicyTarget, String)> =
        Vec::new();


    for target in [
        PolicyTarget::Screensaver,
        PolicyTarget::Wallpaper,
    ] {
        let keys =
            policy_table(
                &document,
                target,
            )?
            .map(
                |table| {
                    table
                        .iter()
                        .map(
                            |(key, _)| key.to_string()
                        )
                        .collect::<Vec<_>>()
                }
            )
            .unwrap_or_default();


        for key in keys {
            let resolved =
                resolved_policy_source_path(
                    &document,
                    target,
                    &key,
                )?;


            if paths_refer_to_same_source(
                &resolved,
                &original_source_path,
            ) {
                matching_policies.push(
                    (
                        target,
                        key,
                    )
                );
            }
        }
    }


    if matching_policies.len() > 1 {
        return Err(
            format!(
                "Shader '{}' has multiple policies referring to the same source path; move canceled to avoid choosing which policy settings should survive",
                destination_name,
            )
        );
    }


    let Some((
        source_target,
        source_key,
    )) =
        matching_policies
            .into_iter()
            .next()
    else {
        // The file can still be moved even if it has no policy.
        return save_document(
            config_path,
            &document,
        );
    };


    if source_target
        == destination_target
    {
        // The destination directory is authoritative.  A policy for a
        // managed shader no longer needs external-path metadata.
        remove_source_path_metadata(
            &mut document,
            source_target,
            &source_key,
        )?;


        return save_document(
            config_path,
            &document,
        );
    }


    // A move between managed shader directories is also a policy-target
    // conversion.  Refuse to overwrite an existing destination policy.
    if let Some(existing_destination_key) =
        matching_policy_key_for_source(
            &document,
            destination_target,
            destination_name,
            destination_path,
        )?
    {
        return Err(
            format!(
                "Cannot change the {} policy for '{}' to {} because destination policy '{}' already refers to the managed destination shader",
                source_target.name(),
                destination_name,
                destination_target.name(),
                existing_destination_key,
            )
        );
    }


    let source_item =
        policy_table(
            &document,
            source_target,
        )?
        .and_then(
            |table| table.get(
                &source_key
            )
        )
        .cloned()
        .ok_or_else(
            || {
                format!(
                    "Policy '{}' disappeared from [{}] while reconciling the shader move",
                    source_key,
                    source_target.table_name(),
                )
            }
        )?;


    let destination_key =
        {
            let destination_table =
                policy_table_mut(
                    &mut document,
                    destination_target,
                )?;


            unique_policy_storage_key(
                destination_table,
                destination_name,
                destination_path,
            )
        };


    {
        let destination_table =
            policy_table_mut(
                &mut document,
                destination_target,
            )?;


        destination_table[
            &destination_key
        ] =
            source_item;
    }


    remove_source_path_metadata(
        &mut document,
        destination_target,
        &destination_key,
    )?;


    {
        let source_table =
            policy_table_mut(
                &mut document,
                source_target,
            )?;


        source_table.remove(
            &source_key
        );
    }


    remove_source_path_metadata(
        &mut document,
        source_target,
        &source_key,
    )?;


    save_document(
        config_path,
        &document,
    )
}


pub fn reconcile_shader_move(
    config_path: &Path,
    shader: &str,
    destination_path: &Path,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;

    if !destination_path.is_absolute() {
        return Err(
            format!(
                "Moved shader path must be absolute: {}",
                destination_path.display(),
            )
        );
    }

    let mut document =
        load_document(
            config_path
        )?;

    let managed_path =
        crate::locate_paths::shader_dir()
            .join(
                &shader
            );

    for target in [
        PolicyTarget::Screensaver,
        PolicyTarget::Wallpaper,
    ] {
        let policy_exists =
            policy_table(
                &document,
                target,
            )?
            .and_then(
                |table| {
                    matching_shader_key(
                        table,
                        &shader,
                    )
                }
            )
            .is_some();

        if !policy_exists {
            continue;
        }

        if destination_path == &managed_path {
            set_source_path_metadata(
                &mut document,
                target,
                &shader,
                None,
            )?;
        } else {
            let destination_text =
                destination_path
                    .to_str()
                    .ok_or_else(
                        || {
                            format!(
                                "Moved shader path is not valid UTF-8: {}",
                                destination_path.display(),
                            )
                        }
                    )?;

            set_source_path_metadata(
                &mut document,
                target,
                &shader,
                Some(
                    destination_text
                ),
            )?;
        }
    }

    save_document(
        config_path,
        &document,
    )
}


/// Return external source paths that belong to actual policies for the
/// requested target.  Stale path-table entries without a corresponding
/// policy are ignored.
pub fn external_policy_paths(
    config_path: &Path,
    target: PolicyTarget,
) -> Result<Vec<(String, PathBuf)>, String> {

    let document =
        load_document(
            config_path
        )?;

    let Some(policy_table) =
        policy_table(
            &document,
            target,
        )?
    else {
        return Ok(
            Vec::new()
        );
    };

    let mut paths =
        Vec::new();

    for (
        shader,
        _,
    ) in policy_table.iter()
    {
        let mut source_path =
            None;

        for table_name in [
            target.path_table_name(),
            target.legacy_path_table_name(),
        ] {
            let Some(item) =
                document.get(
                    table_name
                )
            else {
                continue;
            };

            let table =
                item.as_table()
                    .ok_or_else(
                        || {
                            format!(
                                "[{}] exists but is not a TOML table",
                                table_name,
                            )
                        }
                    )?;

            let Some(existing_key) =
                matching_shader_key(
                    table,
                    shader,
                )
            else {
                continue;
            };

            let raw_path =
                table
                    .get(
                        &existing_key
                    )
                    .and_then(
                        |item| {
                            item.as_value()
                        }
                    )
                    .and_then(
                        |value| {
                            value.as_str()
                        }
                    )
                    .ok_or_else(
                        || {
                            format!(
                                "Shader path '{}' in [{}] must be a TOML string",
                                existing_key,
                                table_name,
                            )
                        }
                    )?;

            let path =
                PathBuf::from(
                    raw_path
                );

            if !path.is_absolute() {
                return Err(
                    format!(
                        "Shader path '{}' in [{}] must be absolute: {}",
                        existing_key,
                        table_name,
                        raw_path,
                    )
                );
            }

            source_path =
                Some(path);

            break;
        }

        if let Some(source_path) =
            source_path
        {
            let display_name =
                source_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        policy_display_name_from_key(shader)
                            .to_string()
                    });

            paths.push(
                (
                    display_name,
                    source_path,
                )
            );
        }
    }

    paths.sort_by(
        |left, right| {
            left.0
                .to_ascii_lowercase()
                .cmp(
                    &right.0
                        .to_ascii_lowercase()
                )
        }
    );

    Ok(
        paths
    )
}


pub fn policy_source_path(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
) -> Result<Option<PathBuf>, String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    let document =
        load_document(
            config_path
        )?;


    for table_name in [
        target.path_table_name(),
        target.legacy_path_table_name(),
    ] {
        let Some(item) =
            document.get(
                table_name
            )
        else {
            continue;
        };


        let table =
            item.as_table()
                .ok_or_else(
                    || {
                        format!(
                            "[{}] exists but is not a TOML table",
                            table_name,
                        )
                    }
                )?;


        let Some(existing_key) =
            matching_shader_key(
                table,
                &shader,
            )
        else {
            continue;
        };


        let raw_path =
            table
                .get(
                    &existing_key
                )
                .and_then(
                    |item| {
                        item.as_value()
                    }
                )
                .and_then(
                    |value| {
                        value.as_str()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Shader path '{}' in [{}] must be a TOML string",
                            existing_key,
                            table_name,
                        )
                    }
                )?;


        let path =
            PathBuf::from(
                raw_path
            );


        if !path.is_absolute() {
            return Err(
                format!(
                    "Shader path '{}' in [{}] must be absolute: {}",
                    existing_key,
                    table_name,
                    raw_path,
                )
            );
        }


        return Ok(
            Some(
                path
            )
        );
    }


    Ok(
        None
    )
}

//
// ------------------------------------------------------------
// TOML document handling
// ------------------------------------------------------------
//

fn load_document(
    path: &Path,
) -> Result<DocumentMut, String> {

    let text =
        fs::read_to_string(
            path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read configuration file {} ({})",
                    path.display(),
                    error,
                )
            }
        )?;


    text.parse::<DocumentMut>()
        .map_err(
            |error| {
                format!(
                    "Unable to parse configuration file {} ({})",
                    path.display(),
                    error,
                )
            }
        )
}


fn save_document(
    path: &Path,
    document: &DocumentMut,
) -> Result<(), String> {

    fs::write(
        path,
        document.to_string(),
    )
    .map_err(
        |error| {
            format!(
                "Unable to write configuration file {} ({})",
                path.display(),
                error,
            )
        }
    )
}


fn policy_table_mut<'a>(
    document: &'a mut DocumentMut,
    target: PolicyTarget,
) -> Result<&'a mut Table, String> {

    let table_name =
        target.table_name();


    if !document.contains_key(
        table_name
    ) {
        let mut table =
            Table::new();

        table.set_implicit(
            false
        );

        document[
            table_name
        ] =
            Item::Table(
                table
            );
    }


    document[
        table_name
    ]
        .as_table_mut()
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn policy_table<'a>(
    document: &'a DocumentMut,
    target: PolicyTarget,
) -> Result<Option<&'a Table>, String> {

    let table_name =
        target.table_name();


    let Some(item) =
        document.get(
            table_name
        )
    else {
        return Ok(
            None
        );
    };


    item.as_table()
        .map(
            Some
        )
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn source_path_table_mut<'a>(
    document: &'a mut DocumentMut,
    target: PolicyTarget,
) -> Result<&'a mut Table, String> {

    let table_name =
        target.path_table_name();


    if !document.contains_key(
        table_name
    ) {
        let mut table =
            Table::new();

        table.set_implicit(
            false
        );

        document[
            table_name
        ] =
            Item::Table(
                table
            );
    }


    document[
        table_name
    ]
        .as_table_mut()
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn source_path_table<'a>(
    document: &'a DocumentMut,
    target: PolicyTarget,
) -> Result<Option<&'a Table>, String> {

    let table_name =
        target.path_table_name();


    let Some(item) =
        document.get(
            table_name
        )
    else {
        return Ok(
            None
        );
    };


    item.as_table()
        .map(
            Some
        )
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn normalize_source_path(
    source_path: Option<&Path>,
) -> Result<Option<String>, String> {

    let Some(source_path) =
        source_path
    else {
        return Ok(
            None
        );
    };


    if !source_path.is_absolute() {
        return Err(
            format!(
                "External shader path must be absolute: {}",
                source_path.display(),
            )
        );
    }


    let source_path =
        source_path
            .to_str()
            .ok_or_else(
                || {
                    format!(
                        "External shader path is not valid UTF-8: {}",
                        source_path.display(),
                    )
                }
            )?
            .trim();


    if source_path.is_empty() {
        return Err(
            "External shader path may not be empty"
                .to_string()
        );
    }


    Ok(
        Some(
            source_path.to_string()
        )
    )
}


fn set_source_path_metadata(
    document: &mut DocumentMut,
    target: PolicyTarget,
    shader: &str,
    source_path: Option<&str>,
) -> Result<(), String> {

    match source_path {
        Some(source_path) => {
            let table =
                source_path_table_mut(
                    document,
                    target,
                )?;


            if let Some(existing_key) =
                matching_shader_key(
                    table,
                    shader,
                )
            {
                table.remove(
                    &existing_key
                );
            }


            table[
                shader
            ] =
                value(
                    source_path
                );


            remove_source_path_metadata_from_named_table(
                document,
                target.legacy_path_table_name(),
                shader,
            )?;
        }

        None => {
            remove_source_path_metadata(
                document,
                target,
                shader,
            )?;
        }
    }


    Ok(())
}


fn remove_source_path_metadata(
    document: &mut DocumentMut,
    target: PolicyTarget,
    shader: &str,
) -> Result<(), String> {

    remove_source_path_metadata_from_named_table(
        document,
        target.path_table_name(),
        shader,
    )?;


    remove_source_path_metadata_from_named_table(
        document,
        target.legacy_path_table_name(),
        shader,
    )
}


fn remove_source_path_metadata_from_named_table(
    document: &mut DocumentMut,
    table_name: &str,
    shader: &str,
) -> Result<(), String> {

    let should_remove_table =
        {
            let Some(item) =
                document.get_mut(
                    table_name
                )
            else {
                return Ok(());
            };


            let table =
                item.as_table_mut()
                    .ok_or_else(
                        || {
                            format!(
                                "[{}] exists but is not a TOML table",
                                table_name,
                            )
                        }
                    )?;


            if let Some(existing_key) =
                matching_shader_key(
                    table,
                    shader,
                )
            {
                table.remove(
                    &existing_key
                );
            }


            table.is_empty()
        };


    if should_remove_table {
        document.remove(
            table_name
        );
    }


    Ok(())
}

//
// ------------------------------------------------------------
// SQLite single-policy helpers
// ------------------------------------------------------------
//

#[derive(Debug)]
struct DatabasePolicyValues {

    texture_mode:
        Option<String>,

    texture_family:
        Option<String>,

    texture_primitives:
        Option<i64>,

    palette_mode:
        Option<String>,

    palette_color:
        Option<String>,

    rendered_fps:
        Option<i64>,

    animation_speed:
        Option<f64>,

    anti_aliasing:
        Option<String>,

    dithering:
        Option<String>,

    color_precision:
        Option<String>,

    render_scale:
        Option<f64>,

    bloom_mode:
        String,

    bloom_intensity:
        f64,

    bloom_threshold:
        f64,

    invert_colors:
        i64,

    flip_horizontal:
        i64,

    flip_vertical:
        i64,

    hue_rotation:
        f64,
}


fn database_shader_id_for_source(
    shader: &str,
    source_path: &Path,
) -> Result<i64, String> {

    let source_path =
        canonical_or_absolute(
            source_path
        )?;


    let filename =
        source_path
            .file_name()
            .and_then(
                |name| {
                    name.to_str()
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "Shader path has no valid UTF-8 filename: {}",
                        source_path.display(),
                    )
                }
            )?;


    if !filename.eq_ignore_ascii_case(
        shader
    ) {

        return Err(
            format!(
                "Shader name '{}' does not match source filename '{}'",
                shader,
                filename,
            )
        );
    }


    let registered_directory =
        source_path
            .parent()
            .ok_or_else(
                || {
                    format!(
                        "Shader path has no parent directory: {}",
                        source_path.display(),
                    )
                }
            )?
            .to_string_lossy()
            .to_string();


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while resolving shader '{}': {}",
                        shader,
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT shader_id
                 FROM shaders
                 WHERE filename = ?1
                   AND source_path = ?2
                 ORDER BY shader_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare shader lookup for '{}': {}",
                        shader,
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                rusqlite::params![
                    filename,
                    registered_directory,
                ],
                |row| {
                    row.get::<_, i64>(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query database shader '{}': {}",
                        shader,
                        error,
                    )
                }
            )?;


    let mut shader_ids =
        Vec::new();


    for row in rows {

        shader_ids.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode database shader '{}': {}",
                        shader,
                        error,
                    )
                }
            )?
        );
    }


    match shader_ids.as_slice() {

        [shader_id] => {
            Ok(
                *shader_id
            )
        }

        [] => {
            Err(
                format!(
                    "Shader '{}' at '{}' is not registered in screenshaver.db",
                    shader,
                    source_path.display(),
                )
            )
        }

        _ => {
            Err(
                format!(
                    "Shader '{}' at '{}' resolves to multiple database rows",
                    shader,
                    source_path.display(),
                )
            )
        }
    }
}


fn database_policy_values(
    properties: &PolicyDefinition,
) -> Result<DatabasePolicyValues, String> {

    let (
        texture_mode,
        texture_family,
        texture_primitives,
    ) =
        match properties.texture
            .as_deref()
            .map(
                |value| {
                    value.trim()
                        .to_ascii_lowercase()
                }
            )
        {

            None => {
                (
                    None,
                    None,
                    None,
                )
            }

            Some(value)
                if value == "random" =>
            {
                (
                    Some(
                        "random".to_string()
                    ),
                    None,
                    None,
                )
            }

            Some(value) => {

                let specification =
                    crate::parse_texture_specification::parse_texture_specification(
                        &value
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Invalid texture policy '{}': {}",
                                value,
                                error,
                            )
                        }
                    )?;


                (
                    Some(
                        "specific".to_string()
                    ),
                    Some(
                        specification.family
                            .name()
                            .to_ascii_lowercase()
                    ),
                    Some(
                        i64::try_from(
                            specification.requested_primitive_count
                        )
                        .map_err(
                            |_| {
                                format!(
                                    "Texture primitive count {} is too large for SQLite storage",
                                    specification.requested_primitive_count,
                                )
                            }
                        )?
                    ),
                )
            }
        };


    let (
        palette_mode,
        palette_color,
    ) =
        match properties.palette
            .as_deref()
            .map(
                |value| {
                    value.trim()
                        .to_ascii_lowercase()
                }
            )
        {

            None => {
                (
                    None,
                    None,
                )
            }

            Some(value)
                if value == "random" =>
            {
                (
                    Some(
                        "random".to_string()
                    ),
                    None,
                )
            }

            Some(value) => {

                let color =
                    crate::palettes::PaletteColor::parse_hex(
                        &value
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Invalid palette policy '{}': {}",
                                value,
                                error,
                            )
                        }
                    )?;


                (
                    Some(
                        "specific".to_string()
                    ),
                    Some(
                        color.to_hex()
                            .to_ascii_lowercase()
                    ),
                )
            }
        };


    let bloom_mode =
        properties.bloom
            .as_deref()
            .unwrap_or(
                "off"
            )
            .trim()
            .to_ascii_lowercase();


    Ok(
        DatabasePolicyValues {

            texture_mode,

            texture_family,

            texture_primitives,

            palette_mode,

            palette_color,

            rendered_fps:
                properties.fps
                    .map(
                        i64::from
                    ),

            animation_speed:
                properties.speed
                    .map(
                        f64::from
                    ),

            anti_aliasing:
                properties.anti_aliasing
                    .as_deref()
                    .map(
                        |value| {
                            value.trim()
                                .to_ascii_lowercase()
                        }
                    ),

            dithering:
                properties.dithering
                    .as_deref()
                    .map(
                        |value| {
                            value.trim()
                                .to_ascii_lowercase()
                        }
                    ),

            color_precision:
                properties.color_precision
                    .as_deref()
                    .map(
                        |value| {
                            value.trim()
                                .to_ascii_lowercase()
                        }
                    ),

            render_scale:
                properties.render_scale
                    .map(
                        f64::from
                    ),

            bloom_mode,

            bloom_intensity:
                properties.bloom_intensity
                    .unwrap_or(
                        crate::render_bloom::BLOOM_INTENSITY_DEFAULT
                    )
                    as f64,

            bloom_threshold:
                properties.bloom_threshold
                    .unwrap_or(
                        crate::render_bloom::BLOOM_THRESHOLD_DEFAULT
                    )
                    as f64,

            invert_colors:
                if properties.invert_colors
                    .unwrap_or(
                        false
                    )
                {
                    1
                } else {
                    0
                },

            flip_horizontal:
                if properties.flip_horizontal
                    .unwrap_or(
                        false
                    )
                {
                    1
                } else {
                    0
                },

            flip_vertical:
                if properties.flip_vertical
                    .unwrap_or(
                        false
                    )
                {
                    1
                } else {
                    0
                },

            hue_rotation:
                properties.hue_rotation
                    .unwrap_or(
                        crate::postprocess_shader::HUE_ROTATION_DEFAULT
                    )
                    as f64,
        }
    )
}


fn database_policy_name_key(
    policy_name: &str,
) -> Result<String, String> {

    let policy_name =
        policy_name.trim();


    let length =
        policy_name
            .chars()
            .count();


    if !(1..=128)
        .contains(
            &length
        )
    {

        return Err(
            format!(
                "Policy Name must contain between 1 and 128 characters; found {}",
                length,
            )
        );
    }


    // Schema V1 requires a stable case-insensitive key.  Rust's Unicode
    // lowercase mapping is used here as the current canonical implementation.
    // A future normalization-library dependency may strengthen this to full
    // Unicode normalization/case folding without changing the SQL schema.
    let key =
        policy_name
            .chars()
            .flat_map(
                |character| {
                    character.to_lowercase()
                }
            )
            .collect::<String>();


    if key.is_empty() {

        return Err(
            "Policy Name produced an empty comparison key"
                .to_string()
        );
    }


    Ok(
        key
    )
}


fn next_database_policy_name(
    connection: &rusqlite::Connection,
    shader: &str,
) -> Result<String, String> {

    let base =
        shader.trim();


    for ordinal in 1_u32..=10_000 {

        let candidate =
            if ordinal == 1 {

                base.to_string()

            } else {

                format!(
                    "{} ({})",
                    base,
                    ordinal,
                )
            };


        if candidate
            .chars()
            .count()
            > 128
        {
            continue;
        }


        let key =
            database_policy_name_key(
                &candidate
            )?;


        let count: i64 =
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM shader_policies
                     WHERE policy_name_key = ?1",
                    rusqlite::params![
                        key
                    ],
                    |row| {
                        row.get(0)
                    },
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to check Policy Name '{}': {}",
                            candidate,
                            error,
                        )
                    }
                )?;


        if count == 0 {

            return Ok(
                candidate
            );
        }
    }


    Err(
        format!(
            "Unable to generate a unique Policy Name for shader '{}'",
            shader,
        )
    )
}


//
// ------------------------------------------------------------
// Policy formatting and validation
// ------------------------------------------------------------
//

fn normalized_shader_name(
    shader: &str,
) -> Result<String, String> {

    let shader =
        shader.trim();


    if shader.is_empty() {
        return Err(
            "Shader name may not be empty"
                .to_string()
        );
    }


    Ok(
        shader.to_string()
    )
}


fn validate_properties(
    target: PolicyTarget,
    properties: &PolicyDefinition,
) -> Result<(), String> {

    if properties.is_empty() {
        return Err(
            "A policy must define at least one property"
                .to_string()
        );
    }


    if let Some(texture) =
        properties.texture.as_deref()
    {
        validate_property_text(
            "texture",
            texture,
        )?;
    }


    if let Some(palette) =
        properties.palette.as_deref()
    {
        validate_property_text(
            "palette",
            palette,
        )?;
    }


    if let Some(fps) =
        properties.fps
    {
        if !(crate::define_constants::MIN_RENDER_FPS
            ..=crate::define_constants::MAX_RENDER_FPS)
            .contains(
                &fps
            )
        {
            return Err(
                format!(
                    "FPS policy {} is outside the supported range {}-{}",
                    fps,
                    crate::define_constants::MIN_RENDER_FPS,
                    crate::define_constants::MAX_RENDER_FPS,
                )
            );
        }
    }


    if let Some(speed) =
        properties.speed
    {
        let (
            minimum,
            maximum,
        ) =
            match target {
                PolicyTarget::Screensaver => (
                    crate::define_constants::SCREENSAVER_SPEED_MIN,
                    crate::define_constants::SCREENSAVER_SPEED_MAX,
                ),

                PolicyTarget::Wallpaper => (
                    crate::define_constants::WALLPAPER_SPEED_MIN,
                    crate::define_constants::WALLPAPER_SPEED_MAX,
                ),

                PolicyTarget::Unassigned => (
                    crate::define_constants::SCREENSAVER_SPEED_MIN
                        .min(
                            crate::define_constants::WALLPAPER_SPEED_MIN
                        ),
                    crate::define_constants::SCREENSAVER_SPEED_MAX
                        .max(
                            crate::define_constants::WALLPAPER_SPEED_MAX
                        ),
                ),
            };


        if !speed.is_finite()
            || !(minimum..=maximum)
                .contains(
                    &speed
                )
        {
            return Err(
                format!(
                    "Speed policy {} for {} is outside the supported range {}-{}",
                    speed,
                    target.name(),
                    minimum,
                    maximum,
                )
            );
        }
    }


    if let Some(render_scale) =
        properties.render_scale
    {
        if !render_scale.is_finite()
            || !(crate::define_constants::RENDER_SCALE_MIN
                ..=crate::define_constants::RENDER_SCALE_MAX)
                .contains(
                    &render_scale
                )
        {
            return Err(
                format!(
                    "Render-scale policy {} is outside the supported range {:.2}-{:.2}",
                    render_scale,
                    crate::define_constants::RENDER_SCALE_MIN,
                    crate::define_constants::RENDER_SCALE_MAX,
                )
            );
        }
    }


    if let Some(value) =
        properties.anti_aliasing.as_deref()
    {
        validate_named_policy_value(
            "anti_aliasing",
            value,
            &["off", "fxaa"],
        )?;
    }

    if let Some(value) =
        properties.dithering.as_deref()
    {
        validate_named_policy_value(
            "dithering",
            value,
            &["off", "subtle"],
        )?;
    }

    if let Some(value) =
        properties.color_precision.as_deref()
    {
        validate_named_policy_value(
            "color_precision",
            value,
            &["auto", "standard", "high"],
        )?;
    }


    if let Some(value) =
        properties.bloom.as_deref()
    {
        validate_named_policy_value(
            "bloom",
            value,
            &["off", "highlight", "audio"],
        )?;
    }


    if let Some(intensity) =
        properties.bloom_intensity
    {
        crate::render_bloom::validate_bloom_intensity(
            intensity
        )
        .map_err(
            |_| {
                format!(
                    "Bloom-intensity policy {} is outside the supported range {:.2}-{:.2}",
                    intensity,
                    crate::render_bloom::BLOOM_INTENSITY_MIN,
                    crate::render_bloom::BLOOM_INTENSITY_MAX,
                )
            }
        )?;
    }


    if let Some(threshold) =
        properties.bloom_threshold
    {
        crate::render_bloom::validate_bloom_threshold(
            threshold
        )
        .map_err(
            |_| {
                format!(
                    "Bloom-threshold policy {} is outside the supported range {:.2}-{:.2}",
                    threshold,
                    crate::render_bloom::BLOOM_THRESHOLD_MIN,
                    crate::render_bloom::BLOOM_THRESHOLD_MAX,
                )
            }
        )?;
    }

    if let Some(hue_rotation) =
        properties.hue_rotation
    {
        crate::postprocess_shader::validate_hue_rotation(
            hue_rotation
        )?;
    }


    Ok(())
}


fn validate_property_text(
    property_name: &str,
    value: &str,
) -> Result<(), String> {

    let value =
        value.trim();


    if value.is_empty() {
        return Err(
            format!(
                "Policy property '{}' requires a value",
                property_name,
            )
        );
    }


    if value.chars()
        .any(
            char::is_whitespace
        )
    {
        return Err(
            format!(
                "Policy property '{}' may not contain whitespace: '{}'",
                property_name,
                value,
            )
        );
    }


    Ok(())
}


fn validate_named_policy_value(
    property_name: &str,
    value: &str,
    supported_values: &[&str],
) -> Result<(), String> {
    validate_property_text(
        property_name,
        value,
    )?;

    let normalized =
        value.trim()
            .to_ascii_lowercase();

    if supported_values.contains(
        &normalized.as_str()
    ) {
        return Ok(());
    }

    Err(
        format!(
            "Unsupported {} policy value '{}'; supported values: {}",
            property_name,
            value,
            supported_values.join(", "),
        )
    )
}


fn format_policy(
    properties: &PolicyDefinition,
) -> String {

    let mut tokens =
        Vec::with_capacity(
            10
        );


    if let Some(texture) =
        properties.texture.as_deref()
    {
        tokens.push(
            format!(
                "texture:{}",
                texture.trim()
                    .to_ascii_lowercase(),
            )
        );
    }


    if let Some(palette) =
        properties.palette.as_deref()
    {
        tokens.push(
            format!(
                "palette:{}",
                palette.trim()
                    .to_ascii_lowercase(),
            )
        );
    }


    if let Some(fps) =
        properties.fps
    {
        tokens.push(
            format!(
                "fps:{}",
                fps,
            )
        );
    }


    if let Some(speed) =
        properties.speed
    {
        tokens.push(
            format!(
                "speed:{}",
                format_speed(
                    speed
                ),
            )
        );
    }


    if let Some(render_scale) =
        properties.render_scale
    {
        tokens.push(
            format!(
                "render_scale:{}",
                format_speed(
                    render_scale
                ),
            )
        );
    }


    if let Some(value) =
        properties.anti_aliasing.as_deref()
    {
        tokens.push(
            format!(
                "anti_aliasing:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    if let Some(value) =
        properties.dithering.as_deref()
    {
        tokens.push(
            format!(
                "dithering:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    if let Some(value) =
        properties.color_precision.as_deref()
    {
        tokens.push(
            format!(
                "color_precision:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    if let Some(value) =
        properties.bloom.as_deref()
    {
        tokens.push(
            format!(
                "bloom:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    if let Some(intensity) =
        properties.bloom_intensity
    {
        tokens.push(
            format!(
                "bloom_intensity:{}",
                format_speed(
                    intensity
                ),
            )
        );
    }


    if let Some(threshold) =
        properties.bloom_threshold
    {
        tokens.push(
            format!(
                "bloom_threshold:{}",
                format_speed(
                    threshold
                ),
            )
        );
    }
    if let Some(invert_colors) = properties.invert_colors {
        tokens.push(format!("invert_colors:{}", invert_colors));
    }

    if let Some(flip_horizontal) = properties.flip_horizontal {
        tokens.push(format!("flip_horizontal:{}", flip_horizontal));
    }

    if let Some(flip_vertical) = properties.flip_vertical {
        tokens.push(format!("flip_vertical:{}", flip_vertical));
    }

    if let Some(hue_rotation) =
        properties.hue_rotation
    {
        tokens.push(
            format!(
                "hue_rotation:{}",
                format_speed(hue_rotation),
            )
        );
    }


    tokens.join(
        " "
    )
}


fn format_speed(
    speed: f32,
) -> String {

    let mut value =
        speed.to_string();


    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }


        if value.ends_with('.') {
            value.push(
                '0'
            );
        }
    }


    value
}


pub fn policy_display_name_from_key(
    key: &str,
) -> &str {

    let Some((name, suffix)) =
        key.rsplit_once(
            POLICY_ID_SEPARATOR
        )
    else {
        return key;
    };

    if suffix.len() == 64
        && suffix
            .bytes()
            .all(
                |byte| byte.is_ascii_hexdigit()
            )
    {
        name
    } else {
        key
    }
}


fn unique_policy_storage_key(
    table: &Table,
    shader: &str,
    source_path: &Path,
) -> String {

    let filename_in_use =
        table
            .iter()
            .any(
                |(key, _)| {
                    policy_display_name_from_key(key)
                        .eq_ignore_ascii_case(
                            shader
                        )
                }
            );

    if !filename_in_use {
        return shader.to_string();
    }

    let mut hasher =
        Sha256::new();

    hasher.update(
        source_path
            .as_os_str()
            .as_encoded_bytes()
    );

    let digest =
        format!(
            "{:x}",
            hasher.finalize()
        );

    format!(
        "{}{}{}",
        shader,
        POLICY_ID_SEPARATOR,
        digest,
    )
}


fn matching_policy_key_for_source(
    document: &DocumentMut,
    target: PolicyTarget,
    shader: &str,
    source_path: &Path,
) -> Result<Option<String>, String> {

    let Some(table) =
        policy_table(
            document,
            target,
        )?
    else {
        return Ok(None);
    };

    for (key, _) in table.iter() {
        if !policy_display_name_from_key(key)
            .eq_ignore_ascii_case(
                shader
            )
        {
            continue;
        }

        let resolved =
            resolved_policy_source_path(
                document,
                target,
                key,
            )?;

        if paths_refer_to_same_source(
            &resolved,
            source_path,
        ) {
            return Ok(
                Some(
                    key.to_string()
                )
            );
        }
    }

    Ok(None)
}


fn resolved_policy_source_path(
    document: &DocumentMut,
    target: PolicyTarget,
    policy_key: &str,
) -> Result<PathBuf, String> {

    for table_name in [
        target.path_table_name(),
        target.legacy_path_table_name(),
    ] {
        let Some(item) =
            document.get(
                table_name
            )
        else {
            continue;
        };

        let table =
            item
                .as_table()
                .ok_or_else(
                    || {
                        format!(
                            "[{}] exists but is not a TOML table",
                            table_name,
                        )
                    }
                )?;

        let Some(existing_key) =
            table
                .iter()
                .find_map(
                    |(key, _)| {
                        if key.eq_ignore_ascii_case(
                            policy_key
                        ) {
                            Some(key.to_string())
                        } else {
                            None
                        }
                    }
                )
        else {
            continue;
        };

        let raw_path =
            table
                .get(
                    &existing_key
                )
                .and_then(
                    |item| item.as_value()
                )
                .and_then(
                    |value| value.as_str()
                )
                .ok_or_else(
                    || {
                        format!(
                            "Shader path '{}' in [{}] must be a TOML string",
                            existing_key,
                            table_name,
                        )
                    }
                )?;

        return Ok(
            PathBuf::from(
                raw_path
            )
        );
    }

    Ok(
        managed_shader_path(
            target,
            policy_display_name_from_key(
                policy_key
            ),
        )
    )
}


fn managed_shader_path(
    _target: PolicyTarget,
    shader: &str,
) -> PathBuf {

    crate::locate_paths::shader_dir()
        .join(shader)
}


fn canonical_or_absolute(
    path: &Path,
) -> Result<PathBuf, String> {

    if let Ok(canonical) =
        path.canonicalize()
    {
        return Ok(canonical);
    }

    if path.is_absolute() {
        return Ok(
            path.to_path_buf()
        );
    }

    std::env::current_dir()
        .map(
            |directory| directory.join(path)
        )
        .map_err(
            |error| {
                format!(
                    "Unable to resolve shader path '{}': {}",
                    path.display(),
                    error,
                )
            }
        )
}


fn paths_refer_to_same_source(
    left: &Path,
    right: &Path,
) -> bool {

    match (
        left.canonicalize(),
        right.canonicalize(),
    ) {
        (Ok(left), Ok(right)) => {
            left == right
        }

        _ => {
            left == right
        }
    }
}


//
// ------------------------------------------------------------
// Policy lookup and display
// ------------------------------------------------------------
//

fn matching_shader_key(
    table: &Table,
    shader: &str,
) -> Option<String> {

    table
        .iter()
        .find_map(
            |(
                key,
                _,
            )| {
                if key.eq_ignore_ascii_case(
                    shader
                ) {
                    Some(
                        key.to_string()
                    )
                } else {
                    None
                }
            }
        )
}


#[derive(Debug)]
struct PolicyRow {

    shader:
        String,

    texture:
        String,

    palette:
        String,

    fps:
        String,

    speed:
        String,

    render_scale:
        String,

    anti_aliasing:
        String,

    dithering:
        String,

    color_precision:
        String,

    bloom:
        String,

    bloom_intensity:
        String,

    bloom_threshold:
        String,
}


#[derive(Debug)]
struct PolicyTableLayout {

    shader_width:
        usize,

    texture_width:
        usize,

    palette_width:
        usize,

    fps_width:
        usize,

    speed_width:
        usize,

    render_scale_width:
        usize,

    anti_aliasing_width:
        usize,

    dithering_width:
        usize,

    color_precision_width:
        usize,

    bloom_width:
        usize,

    bloom_intensity_width:
        usize,

    bloom_threshold_width:
        usize,
}


fn print_policy_table(
    document: &DocumentMut,
    target: PolicyTarget,
) -> Result<(), String> {

    println!(
        "{} Policies",
        display_target_name(
            target
        )
    );

    println!();


    let Some(table) =
        policy_table(
            document,
            target,
        )?
    else {
        println!(
            "No {} policies defined.",
            target.name(),
        );

        return Ok(());
    };


    let rows =
        collect_policy_rows(
            table,
            target,
        )?;


    if rows.is_empty() {
        println!(
            "No {} policies defined.",
            target.name(),
        );

        return Ok(());
    }


    let layout =
        calculate_policy_table_layout(
            &rows
        );


    print_policy_table_header(
        &layout
    );


    for row in
        &rows
    {
        println!(
            "{:<shader_width$}  {:<texture_width$}  {:<palette_width$}  {:>fps_width$}  {:>speed_width$}  {:>render_scale_width$}  {:<anti_aliasing_width$}  {:<dithering_width$}  {:<color_precision_width$}  {:<bloom_width$}  {:>bloom_intensity_width$}  {:>bloom_threshold_width$}",
            row.shader,
            row.texture,
            row.palette,
            row.fps,
            row.speed,
            row.render_scale,
            row.anti_aliasing,
            row.dithering,
            row.color_precision,
            row.bloom,
            row.bloom_intensity,
            row.bloom_threshold,
            shader_width = layout.shader_width,
            texture_width = layout.texture_width,
            palette_width = layout.palette_width,
            fps_width = layout.fps_width,
            speed_width = layout.speed_width,
            render_scale_width = layout.render_scale_width,
            anti_aliasing_width = layout.anti_aliasing_width,
            dithering_width = layout.dithering_width,
            color_precision_width = layout.color_precision_width,
            bloom_width = layout.bloom_width,
            bloom_intensity_width = layout.bloom_intensity_width,
            bloom_threshold_width = layout.bloom_threshold_width,
        );
    }


    Ok(())
}


fn collect_policy_rows(
    table: &Table,
    target: PolicyTarget,
) -> Result<Vec<PolicyRow>, String> {

    let mut entries =
        table.iter()
            .collect::<Vec<_>>();


    entries.sort_by(
        |(
            left,
            _,
        ),
         (
            right,
            _,
        )| {
            left.to_ascii_lowercase()
                .cmp(
                    &right.to_ascii_lowercase()
                )
        }
    );


    entries
        .into_iter()
        .map(
            |(
                shader,
                item,
            )| {
                let specification =
                    item.as_str()
                        .ok_or_else(
                            || {
                                format!(
                                    "Policy '{}' in [{}] is not a string",
                                    shader,
                                    target.table_name(),
                                )
                            }
                        )?;


                let speed =
                    match policy_property_value(
                        specification,
                        "speed",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid speed '{}' for policy '{}' in [{}]",
                                                value,
                                                shader,
                                                target.table_name(),
                                            )
                                        }
                                    )?;


                            format!(
                                "{:.3}",
                                parsed,
                            )
                        }

                        None => {
                            "-".to_string()
                        }
                    };


                let render_scale =
                    match policy_property_value(
                        specification,
                        "render_scale",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid render_scale '{}' for policy '{}' in [{}]",
                                                value,
                                                shader,
                                                target.table_name(),
                                            )
                                        }
                                    )?;

                            if !parsed.is_finite()
                                || !(crate::define_constants::RENDER_SCALE_MIN
                                    ..=crate::define_constants::RENDER_SCALE_MAX)
                                    .contains(&parsed)
                            {
                                return Err(
                                    format!(
                                        "render_scale '{}' for policy '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                                        value,
                                        shader,
                                        target.table_name(),
                                        crate::define_constants::RENDER_SCALE_MIN,
                                        crate::define_constants::RENDER_SCALE_MAX,
                                    )
                                );
                            }

                            format!(
                                "{:.3}",
                                parsed,
                            )
                        }

                        None => {
                            "-".to_string()
                        }
                    };


                let bloom_intensity =
                    match policy_property_value(
                        specification,
                        "bloom_intensity",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid bloom_intensity '{}' for policy '{}' in [{}]",
                                                value,
                                                shader,
                                                target.table_name(),
                                            )
                                        }
                                    )?;

                            crate::render_bloom::validate_bloom_intensity(
                                parsed
                            )
                            .map_err(
                                |_| {
                                    format!(
                                        "bloom_intensity '{}' for policy '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                                        value,
                                        shader,
                                        target.table_name(),
                                        crate::render_bloom::BLOOM_INTENSITY_MIN,
                                        crate::render_bloom::BLOOM_INTENSITY_MAX,
                                    )
                                }
                            )?;

                            format!(
                                "{:.3}",
                                parsed,
                            )
                        }

                        None => {
                            "-".to_string()
                        }
                    };


                let bloom_threshold =
                    match policy_property_value(
                        specification,
                        "bloom_threshold",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid bloom_threshold '{}' for policy '{}' in [{}]",
                                                value,
                                                shader,
                                                target.table_name(),
                                            )
                                        }
                                    )?;

                            crate::render_bloom::validate_bloom_threshold(
                                parsed
                            )
                            .map_err(
                                |_| {
                                    format!(
                                        "bloom_threshold '{}' for policy '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                                        value,
                                        shader,
                                        target.table_name(),
                                        crate::render_bloom::BLOOM_THRESHOLD_MIN,
                                        crate::render_bloom::BLOOM_THRESHOLD_MAX,
                                    )
                                }
                            )?;

                            format!(
                                "{:.3}",
                                parsed,
                            )
                        }

                        None => {
                            "-".to_string()
                        }
                    };


                Ok(
                    PolicyRow {
                        shader:
                            shader.to_string(),

                        texture:
                            policy_property_value(
                                specification,
                                "texture",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        palette:
                            policy_property_value(
                                specification,
                                "palette",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        fps:
                            policy_property_value(
                                specification,
                                "fps",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        speed,

                        render_scale,

                        anti_aliasing:
                            policy_property_value(
                                specification,
                                "anti_aliasing",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        dithering:
                            policy_property_value(
                                specification,
                                "dithering",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        color_precision:
                            policy_property_value(
                                specification,
                                "color_precision",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        bloom:
                            policy_property_value(
                                specification,
                                "bloom",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        bloom_intensity,

                        bloom_threshold,
                    }
                )
            }
        )
        .collect()
}


fn calculate_policy_table_layout(
    rows: &[PolicyRow],
) -> PolicyTableLayout {

    PolicyTableLayout {
        shader_width:
            rows.iter()
                .map(
                    |row| {
                        row.shader.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Shader".len()
                ),

        texture_width:
            rows.iter()
                .map(
                    |row| {
                        row.texture.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Texture".len()
                ),

        palette_width:
            rows.iter()
                .map(
                    |row| {
                        row.palette.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Palette".len()
                ),

        fps_width:
            rows.iter()
                .map(
                    |row| {
                        row.fps.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "FPS".len()
                ),

        speed_width:
            rows.iter()
                .map(
                    |row| {
                        row.speed.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Speed".len()
                ),

        render_scale_width:
            rows.iter()
                .map(
                    |row| {
                        row.render_scale.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Render Scale".len()
                ),

        anti_aliasing_width:
            rows.iter()
                .map(
                    |row| {
                        row.anti_aliasing.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Anti-aliasing".len()
                ),

        dithering_width:
            rows.iter()
                .map(
                    |row| {
                        row.dithering.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Dithering".len()
                ),

        color_precision_width:
            rows.iter()
                .map(
                    |row| {
                        row.color_precision.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Color precision".len()
                ),

        bloom_width:
            rows.iter()
                .map(
                    |row| {
                        row.bloom.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Bloom".len()
                ),

        bloom_intensity_width:
            rows.iter()
                .map(
                    |row| {
                        row.bloom_intensity.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Bloom intensity".len()
                ),

        bloom_threshold_width:
            rows.iter()
                .map(
                    |row| {
                        row.bloom_threshold.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Bloom threshold".len()
                ),

    }
}


fn print_policy_table_header(
    layout: &PolicyTableLayout,
) {

    println!(
        "{:<shader_width$}  {:<texture_width$}  {:<palette_width$}  {:>fps_width$}  {:>speed_width$}  {:>render_scale_width$}  {:<anti_aliasing_width$}  {:<dithering_width$}  {:<color_precision_width$}  {:<bloom_width$}  {:>bloom_intensity_width$}  {:>bloom_threshold_width$}",
        "Shader",
        "Texture",
        "Palette",
        "FPS",
        "Speed",
        "Render Scale",
        "Anti-aliasing",
        "Dithering",
        "Color precision",
        "Bloom",
        "Bloom intensity",
        "Bloom threshold",
        shader_width = layout.shader_width,
        texture_width = layout.texture_width,
        palette_width = layout.palette_width,
        fps_width = layout.fps_width,
        speed_width = layout.speed_width,
        render_scale_width = layout.render_scale_width,
        anti_aliasing_width = layout.anti_aliasing_width,
        dithering_width = layout.dithering_width,
        color_precision_width = layout.color_precision_width,
        bloom_width = layout.bloom_width,
        bloom_intensity_width = layout.bloom_intensity_width,
        bloom_threshold_width = layout.bloom_threshold_width,
    );


    println!(
        "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
        "-".repeat(
            layout.shader_width
        ),
        "-".repeat(
            layout.texture_width
        ),
        "-".repeat(
            layout.palette_width
        ),
        "-".repeat(
            layout.fps_width
        ),
        "-".repeat(
            layout.speed_width
        ),
        "-".repeat(
            layout.render_scale_width
        ),
        "-".repeat(
            layout.anti_aliasing_width
        ),
        "-".repeat(
            layout.dithering_width
        ),
        "-".repeat(
            layout.color_precision_width
        ),
        "-".repeat(
            layout.bloom_width
        ),
        "-".repeat(
            layout.bloom_intensity_width
        ),
        "-".repeat(
            layout.bloom_threshold_width
        ),
    );
}


fn policy_property_value<'a>(
    specification: &'a str,
    property: &str,
) -> Option<&'a str> {

    specification
        .split_whitespace()
        .find_map(
            |token| {
                let (
                    name,
                    value,
                ) =
                    token.split_once(
                        ':'
                    )?;


                if name.eq_ignore_ascii_case(
                    property
                ) {
                    Some(
                        value
                    )
                } else {
                    None
                }
            }
        )
}


fn display_target_name(
    target: PolicyTarget,
) -> &'static str {

    match target {
        PolicyTarget::Screensaver => {
            "Screensaver"
        }

        PolicyTarget::Wallpaper => {
            "Wallpaper"
        }

        PolicyTarget::Unassigned => {
            "Unassigned"
        }
    }
}

