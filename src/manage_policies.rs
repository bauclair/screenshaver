use std::path::{Path, PathBuf};

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



}


/// Returns true for either canonical default.glsl fallback policy.
/// These policies guarantee at least one Screensaver and Wallpaper target.
pub fn is_protected_default_policy(
    policy_id: i64,
) -> Result<bool, String> {
    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while checking protected policy ID {}: {}",
                        policy_id,
                        error,
                    )
                }
            )?;

    is_protected_default_policy_in_connection(
        &connection,
        policy_id,
    )
}


fn is_protected_default_policy_in_connection(
    connection: &rusqlite::Connection,
    policy_id: i64,
) -> Result<bool, String> {
    let managed_source_path =
        crate::locate_paths::shader_dir()
            .to_string_lossy()
            .to_string();

    let count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_id = ?1
                   AND lower(s.filename) = 'default.glsl'
                   AND s.source_path = ?2
                   AND p.policy_target IN ('screensaver', 'wallpaper')
                   AND p.policy_id = (
                       SELECT MIN(p2.policy_id)
                       FROM shader_policies AS p2
                       JOIN shaders AS s2
                         ON s2.shader_id = p2.shader_id
                       WHERE lower(s2.filename) = 'default.glsl'
                         AND s2.source_path = ?2
                         AND p2.policy_target = p.policy_target
                   )",
                rusqlite::params![
                    policy_id,
                    managed_source_path,
                ],
                |row| row.get(0),
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to determine whether policy ID {} is a protected fallback policy: {}",
                        policy_id,
                        error,
                    )
                }
            )?;

    Ok(count == 1)
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


#[derive(
    Debug,
    Clone,
    Copy,
    Default,
)]
pub struct BulkPolicyFieldMask {
    pub policy_target: bool,
    pub texture: bool,
    pub palette: bool,
    pub fps: bool,
    pub speed: bool,
    pub render_scale: bool,
    pub anti_aliasing: bool,
    pub dithering: bool,
    pub color_precision: bool,
    pub bloom: bool,
    pub bloom_intensity: bool,
    pub bloom_threshold: bool,
    pub invert_colors: bool,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub hue_rotation: bool,
}


#[derive(
    Debug,
    Clone,
)]
pub struct BulkPolicyPatch {
    pub policy_id: i64,
    pub current_target: PolicyTarget,
    pub destination_target: Option<PolicyTarget>,
    pub policy_key: String,
    pub properties: PolicyDefinition,
    pub fields: BulkPolicyFieldMask,
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

pub fn retarget_policy_by_id(
    policy_id: i64,
    destination_target: PolicyTarget,
) -> Result<(), String> {
    let mut connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while changing target for policy ID {}: {}", policy_id, error))?;
    let transaction = connection.transaction()
        .map_err(|error| format!("Unable to begin retarget transaction for policy ID {}: {}", policy_id, error))?;

    let (stored_name, stored_target): (String, String) = transaction.query_row(
        "SELECT policy_name, policy_target FROM shader_policies WHERE policy_id = ?1",
        [policy_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|error| format!("Unable to locate policy ID {} for retargeting: {}", policy_id, error))?;

    let current_target = PolicyTarget::parse(&stored_target)?;
    if current_target == destination_target { return Ok(()); }

    if is_protected_default_policy_in_connection(&transaction, policy_id)? {
        return Err(format!("Policy Target for protected fallback policy '{}' cannot be changed", stored_name));
    }

    let changed = transaction.execute(
        "UPDATE shader_policies SET policy_target = ?1 WHERE policy_id = ?2",
        rusqlite::params![destination_target.name(), policy_id],
    ).map_err(|error| format!("Unable to change Policy Target for policy ID {} ('{}'): {}", policy_id, stored_name, error))?;

    if changed != 1 {
        return Err(format!("Expected to retarget policy ID {}, updated {}", policy_id, changed));
    }

    transaction.commit().map_err(|error| format!("Unable to commit Policy Target change for policy ID {}: {}", policy_id, error))
}

pub fn assign_unassigned_policies_by_id(
    policy_ids: &[i64],
    destination_target: PolicyTarget,
) -> Result<usize, String> {
    if destination_target == PolicyTarget::Unassigned {
        return Err("Unassigned policies must be assigned to Screensaver or Wallpaper".to_string());
    }

    let mut connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while assigning Unassigned policies: {}", error))?;
    let transaction = connection.transaction()
        .map_err(|error| format!("Unable to begin Unassigned-policy assignment transaction: {}", error))?;
    let mut changed = 0_usize;

    for &policy_id in policy_ids {
        let (filename, stored_name, stored_target): (String, String, String) = transaction.query_row(
            "SELECT s.filename, p.policy_name, p.policy_target FROM shader_policies AS p JOIN shaders AS s ON s.shader_id = p.shader_id WHERE p.policy_id = ?1",
            [policy_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(|error| format!("Unable to locate policy ID {} while assigning Unassigned policies: {}", policy_id, error))?;

        if !stored_target.eq_ignore_ascii_case("unassigned") {
            return Err(format!("Policy ID {} ('{}') is no longer Unassigned", policy_id, stored_name));
        }

        let generated_unassigned_name = format!("{} (unassigned)", filename);
        let updated = if stored_name.eq_ignore_ascii_case(&generated_unassigned_name) {
            let desired_name = format!("{} ({})", filename, destination_target.name());
            let desired_key = database_policy_name_key(&desired_name)?;
            transaction.execute(
                "UPDATE shader_policies SET policy_target = ?1, policy_name = ?2, policy_name_key = ?3 WHERE policy_id = ?4 AND policy_target = 'unassigned'",
                rusqlite::params![destination_target.name(), desired_name, desired_key, policy_id],
            )
        } else {
            transaction.execute(
                "UPDATE shader_policies SET policy_target = ?1 WHERE policy_id = ?2 AND policy_target = 'unassigned'",
                rusqlite::params![destination_target.name(), policy_id],
            )
        }.map_err(|error| format!("Unable to assign policy ID {} ('{}'): {}", policy_id, stored_name, error))?;

        if updated != 1 { return Err(format!("Expected to assign policy ID {}, updated {}", policy_id, updated)); }
        changed += 1;
    }

    transaction.commit().map_err(|error| format!("Unable to commit Unassigned-policy assignment transaction: {}", error))?;
    Ok(changed)
}



pub fn suggested_clone_policy_name(
    policy_id: i64,
) -> Result<String, String> {
    let connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while generating clone name for policy ID {}: {}", policy_id, error))?;

    let (source_name, target): (String, String) = connection.query_row(
        "SELECT policy_name, policy_target FROM shader_policies WHERE policy_id = ?1",
        [policy_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|error| format!("Unable to locate policy ID {} while generating clone name: {}", policy_id, error))?;

    let base = source_name.trim();
    for ordinal in 2_u32..=10_000 {
        let candidate = format!("{} ({})", base, ordinal);
        if candidate.chars().count() > 128 { continue; }
        let candidate_key = database_policy_name_key(&candidate)?;
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM shader_policies WHERE policy_name_key = ?1 AND policy_target = ?2",
            rusqlite::params![candidate_key, target],
            |row| row.get(0),
        ).map_err(|error| format!("Unable to check suggested clone name '{}': {}", candidate, error))?;
        if count == 0 { return Ok(candidate); }
    }

    Err(format!("Unable to generate a clone name for policy ID {} ('{}')", policy_id, source_name))
}


pub fn clone_policy_by_id(
    source_policy_id: i64,
    new_policy_name: &str,
) -> Result<i64, String> {
    let destination_name = new_policy_name.trim();
    let destination_key = database_policy_name_key(destination_name)?;

    let mut connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while cloning policy ID {}: {}", source_policy_id, error))?;
    let transaction = connection.transaction()
        .map_err(|error| format!("Unable to begin clone transaction for policy ID {}: {}", source_policy_id, error))?;

    #[derive(Debug)]
    struct CloneSource {
        shader_id: i64, policy_target: String, texture_mode: Option<String>, texture_family: Option<String>,
        texture_primitives: Option<i64>, palette_mode: Option<String>, palette_color: Option<String>,
        rendered_fps: Option<i64>, animation_speed: Option<f64>, anti_aliasing: Option<String>, dithering: Option<String>,
        color_precision: Option<String>, render_scale: Option<f64>, bloom_mode: Option<String>, bloom_intensity: Option<f64>,
        bloom_threshold: Option<f64>, invert_colors: Option<i64>, flip_horizontal: Option<i64>, flip_vertical: Option<i64>, hue_rotation: Option<f64>,
    }

    let source = transaction.query_row(
        "SELECT shader_id, policy_target, texture_mode, texture_family, texture_primitives, palette_mode, palette_color, rendered_fps, animation_speed, anti_aliasing, dithering, color_precision, render_scale, bloom_mode, bloom_intensity, bloom_threshold, invert_colors, flip_horizontal, flip_vertical, hue_rotation FROM shader_policies WHERE policy_id = ?1",
        [source_policy_id],
        |row| Ok(CloneSource { shader_id: row.get(0)?, policy_target: row.get(1)?, texture_mode: row.get(2)?, texture_family: row.get(3)?, texture_primitives: row.get(4)?, palette_mode: row.get(5)?, palette_color: row.get(6)?, rendered_fps: row.get(7)?, animation_speed: row.get(8)?, anti_aliasing: row.get(9)?, dithering: row.get(10)?, color_precision: row.get(11)?, render_scale: row.get(12)?, bloom_mode: row.get(13)?, bloom_intensity: row.get(14)?, bloom_threshold: row.get(15)?, invert_colors: row.get(16)?, flip_horizontal: row.get(17)?, flip_vertical: row.get(18)?, hue_rotation: row.get(19)? }),
    ).map_err(|error| format!("Unable to read source policy ID {} before cloning: {}", source_policy_id, error))?;

    let inserted = transaction.execute(
        "INSERT INTO shader_policies (policy_name, policy_name_key, shader_id, policy_target, texture_mode, texture_family, texture_primitives, palette_mode, palette_color, rendered_fps, animation_speed, anti_aliasing, dithering, color_precision, render_scale, bloom_mode, bloom_intensity, bloom_threshold, invert_colors, flip_horizontal, flip_vertical, hue_rotation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
        rusqlite::params![destination_name, destination_key, source.shader_id, source.policy_target, source.texture_mode, source.texture_family, source.texture_primitives, source.palette_mode, source.palette_color, source.rendered_fps, source.animation_speed, source.anti_aliasing, source.dithering, source.color_precision, source.render_scale, source.bloom_mode, source.bloom_intensity, source.bloom_threshold, source.invert_colors, source.flip_horizontal, source.flip_vertical, source.hue_rotation],
    ).map_err(|error| format!("Unable to clone policy ID {} as '{}': {}", source_policy_id, destination_name, error))?;
    if inserted != 1 { return Err(format!("Expected to clone one policy ID {}, cloned {}", source_policy_id, inserted)); }
    let new_policy_id = transaction.last_insert_rowid();
    transaction.commit().map_err(|error| format!("Unable to commit clone of policy ID {}: {}", source_policy_id, error))?;
    Ok(new_policy_id)
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


    let policy_name =
        next_database_policy_name(
            &transaction,
            &shader,
            target,
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



pub fn rename_policy_by_id(
    policy_id: i64,
    new_policy_name: &str,
) -> Result<(), String> {
    let destination_name = new_policy_name.trim();
    let destination_key = database_policy_name_key(destination_name)?;
    let connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while renaming policy ID {}: {}", policy_id, error))?;
    let changed = connection.execute(
        "UPDATE shader_policies SET policy_name = ?1, policy_name_key = ?2 WHERE policy_id = ?3",
        rusqlite::params![destination_name, destination_key, policy_id],
    ).map_err(|error| format!("Unable to rename policy ID {} as '{}': {}", policy_id, destination_name, error))?;
    match changed { 1 => Ok(()), 0 => Err(format!("Policy ID {} no longer exists", policy_id)), count => Err(format!("Policy ID {} unexpectedly matched {} rows", policy_id, count)) }
}

pub fn replace_policy_by_id(
    policy_id: i64,
    properties: PolicyDefinition,
) -> Result<(), String> {
    let connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while replacing policy ID {}: {}", policy_id, error))?;
    let (stored_name, stored_target): (String, String) = connection.query_row(
        "SELECT policy_name, policy_target FROM shader_policies WHERE policy_id = ?1",
        [policy_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|error| format!("Unable to locate policy ID {} before save: {}", policy_id, error))?;
    let target = PolicyTarget::parse(&stored_target)?;
    validate_properties(target, &properties)?;
    let values = database_policy_values(&properties)?;
    let changed = connection.execute(
        "UPDATE shader_policies SET texture_mode = ?1, texture_family = ?2, texture_primitives = ?3, palette_mode = ?4, palette_color = ?5, rendered_fps = ?6, animation_speed = ?7, anti_aliasing = ?8, dithering = ?9, color_precision = ?10, render_scale = ?11, bloom_mode = ?12, bloom_intensity = ?13, bloom_threshold = ?14, invert_colors = ?15, flip_horizontal = ?16, flip_vertical = ?17, hue_rotation = ?18 WHERE policy_id = ?19",
        rusqlite::params![values.texture_mode, values.texture_family, values.texture_primitives, values.palette_mode, values.palette_color, values.rendered_fps, values.animation_speed, values.anti_aliasing, values.dithering, values.color_precision, values.render_scale, values.bloom_mode, values.bloom_intensity, values.bloom_threshold, values.invert_colors, values.flip_horizontal, values.flip_vertical, values.hue_rotation, policy_id],
    ).map_err(|error| format!("Unable to update policy ID {} ('{}'): {}", policy_id, stored_name, error))?;
    match changed { 1 => Ok(()), 0 => Err(format!("Policy ID {} no longer exists", policy_id)), count => Err(format!("Policy ID {} unexpectedly matched {} rows", policy_id, count)) }
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
    _config_path: &Path,
    creations: &[BulkPolicyCreation],
) -> Result<BulkPolicyCreationResult, String> {

    if creations.is_empty() {
        return Ok(
            BulkPolicyCreationResult::default()
        );
    }


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


        if policy_exists_for_source(
            Path::new(""),
            creation.target,
            &shader,
            &creation.source_path,
        )? {
            result.skipped_existing +=
                1;

            continue;
        }


        add_policy_for_source(
            Path::new(""),
            creation.target,
            &shader,
            creation.properties.clone(),
            &creation.source_path,
        )?;


        result.created +=
            1;
    }


    Ok(
        result
    )
}


pub fn patch_policies_by_id(
    patches: &[BulkPolicyPatch],
) -> Result<usize, String> {
    let mut connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database for Bulk Edit: {}", error))?;
    let transaction = connection.transaction()
        .map_err(|error| format!("Unable to begin Bulk Edit transaction: {}", error))?;
    let mut changed_policies = 0_usize;

    for patch in patches {
        if !patch.properties.is_empty() {
            validate_properties(patch.destination_target.unwrap_or(patch.current_target), &patch.properties)?;
        }
        let (stored_name, stored_target): (String, String) = transaction.query_row(
            "SELECT policy_name, policy_target FROM shader_policies WHERE policy_id = ?1",
            [patch.policy_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|error| format!("Unable to locate policy ID {} for Bulk Edit: {}", patch.policy_id, error))?;
        let current_target = PolicyTarget::parse(&stored_target)?;
        if current_target != patch.current_target {
            return Err(format!("Policy ID {} ('{}') changed target before Bulk Edit could be saved", patch.policy_id, stored_name));
        }
        let values = database_policy_values(&patch.properties)?;
        let protected = is_protected_default_policy_in_connection(&transaction, patch.policy_id)?;
        let mut policy_changed = false;

        if patch.fields.texture { transaction.execute("UPDATE shader_policies SET texture_mode=?1, texture_family=?2, texture_primitives=?3 WHERE policy_id=?4", rusqlite::params![values.texture_mode, values.texture_family, values.texture_primitives, patch.policy_id]).map_err(|e| format!("Unable to update texture for policy ID {}: {}", patch.policy_id,e))?; policy_changed=true; }
        if patch.fields.palette { transaction.execute("UPDATE shader_policies SET palette_mode=?1, palette_color=?2 WHERE policy_id=?3", rusqlite::params![values.palette_mode, values.palette_color, patch.policy_id]).map_err(|e| format!("Unable to update palette for policy ID {}: {}", patch.policy_id,e))?; policy_changed=true; }
        macro_rules! update_one { ($flag:expr,$column:literal,$value:expr) => { if $flag { transaction.execute(concat!("UPDATE shader_policies SET ",$column," = ?1 WHERE policy_id = ?2"), rusqlite::params![$value, patch.policy_id]).map_err(|e| format!("Unable to update {} for policy ID {}: {}", $column, patch.policy_id,e))?; policy_changed=true; } }; }
        update_one!(patch.fields.fps, "rendered_fps", values.rendered_fps);
        update_one!(patch.fields.speed, "animation_speed", values.animation_speed);
        update_one!(patch.fields.render_scale, "render_scale", values.render_scale);
        update_one!(patch.fields.anti_aliasing, "anti_aliasing", values.anti_aliasing);
        update_one!(patch.fields.dithering, "dithering", values.dithering);
        update_one!(patch.fields.color_precision, "color_precision", values.color_precision);
        update_one!(patch.fields.bloom, "bloom_mode", values.bloom_mode);
        update_one!(patch.fields.bloom_intensity, "bloom_intensity", values.bloom_intensity);
        update_one!(patch.fields.bloom_threshold, "bloom_threshold", values.bloom_threshold);
        update_one!(patch.fields.invert_colors, "invert_colors", values.invert_colors);
        update_one!(patch.fields.flip_horizontal, "flip_horizontal", values.flip_horizontal);
        update_one!(patch.fields.flip_vertical, "flip_vertical", values.flip_vertical);
        update_one!(patch.fields.hue_rotation, "hue_rotation", values.hue_rotation);

        if patch.fields.policy_target {
            let destination = patch.destination_target.ok_or_else(|| format!("Bulk Edit marked Policy Target changed for policy ID {}, but no destination target was supplied", patch.policy_id))?;
            if !protected && destination != current_target {
                transaction.execute("UPDATE shader_policies SET policy_target=?1 WHERE policy_id=?2", rusqlite::params![destination.name(), patch.policy_id]).map_err(|e| format!("Unable to change Policy Target for policy ID {}: {}", patch.policy_id,e))?;
                policy_changed=true;
            }
        }
        if policy_changed { changed_policies += 1; }
    }
    transaction.commit().map_err(|error| format!("Unable to commit Bulk Edit transaction: {}", error))?;
    Ok(changed_policies)
}



pub fn delete_policy_by_id(
    _config_path: &Path,
    policy_id: i64,
) -> Result<(), String> {
    let connection = crate::open_database::open()
        .map_err(|error| format!("Unable to open database while deleting policy ID {}: {}", policy_id, error))?;
    let stored_name: String = connection.query_row(
        "SELECT policy_name FROM shader_policies WHERE policy_id = ?1",
        [policy_id],
        |row| row.get(0),
    ).map_err(|error| format!("Unable to locate policy ID {} before deletion: {}", policy_id, error))?;
    if is_protected_default_policy_in_connection(&connection, policy_id)? {
        return Err(format!("Protected fallback policy '{}' cannot be deleted", stored_name));
    }
    let deleted = connection.execute("DELETE FROM shader_policies WHERE policy_id = ?1", [policy_id])
        .map_err(|error| format!("Unable to delete policy ID {} ('{}'): {}", policy_id, stored_name, error))?;
    match deleted { 1 => Ok(()), 0 => Err(format!("Policy ID {} does not exist", policy_id)), count => Err(format!("Deleting policy ID {} unexpectedly removed {} rows", policy_id, count)) }
}

pub fn reconcile_shader_move_from_source(
    _config_path: &Path,
    original_source_path: &Path,
    destination_path: &Path,
    _destination_target: PolicyTarget,
) -> Result<(), String> {

    let original_source_path =
        canonical_or_absolute(
            original_source_path
        )?;


    let destination_path =
        if destination_path.is_absolute() {
            destination_path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(
                    |error| {
                        format!(
                            "Unable to resolve current directory while reconciling shader move: {}",
                            error,
                        )
                    }
                )?
                .join(
                    destination_path
                )
        };


    let original_filename =
        original_source_path
            .file_name()
            .and_then(
                |name| name.to_str()
            )
            .ok_or_else(
                || {
                    format!(
                        "Original shader filename is not valid UTF-8: {}",
                        original_source_path.display(),
                    )
                }
            )?;


    let original_directory =
        original_source_path
            .parent()
            .ok_or_else(
                || {
                    format!(
                        "Original shader path has no parent directory: {}",
                        original_source_path.display(),
                    )
                }
            )?
            .to_string_lossy()
            .to_string();


    let destination_filename =
        destination_path
            .file_name()
            .and_then(
                |name| name.to_str()
            )
            .ok_or_else(
                || {
                    format!(
                        "Destination shader filename is not valid UTF-8: {}",
                        destination_path.display(),
                    )
                }
            )?;


    let destination_directory =
        destination_path
            .parent()
            .ok_or_else(
                || {
                    format!(
                        "Destination shader path has no parent directory: {}",
                        destination_path.display(),
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
                        "Unable to open database while reconciling shader move: {}",
                        error,
                    )
                }
            )?;


    let changed =
        connection
            .execute(
                "UPDATE shaders
                 SET filename = ?1,
                     source_path = ?2
                 WHERE filename = ?3
                   AND source_path = ?4",
                rusqlite::params![
                    destination_filename,
                    destination_directory,
                    original_filename,
                    original_directory,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to update shader registry after moving '{}' to '{}': {}",
                        original_source_path.display(),
                        destination_path.display(),
                        error,
                    )
                }
            )?;


    match changed {
        1 => Ok(()),

        0 => Err(
            format!(
                "Shader '{}' is not registered in screenshaver.db",
                original_source_path.display(),
            )
        ),

        count => Err(
            format!(
                "Shader move unexpectedly updated {} database rows for '{}'",
                count,
                original_source_path.display(),
            )
        ),
    }
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


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while locating shader '{}' for move reconciliation: {}",
                        shader,
                        error,
                    )
                }
            )?;


    let source_directory: String =
        connection
            .query_row(
                "SELECT source_path
                 FROM shaders
                 WHERE filename = ?1
                 ORDER BY shader_id
                 LIMIT 1",
                rusqlite::params![
                    shader
                ],
                |row| row.get(0),
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to locate shader '{}' in screenshaver.db: {}",
                        shader,
                        error,
                    )
                }
            )?;


    let source_path =
        PathBuf::from(
            source_directory
        )
        .join(
            &shader
        );


    reconcile_shader_move_from_source(
        config_path,
        &source_path,
        destination_path,
        PolicyTarget::Unassigned,
    )
}


/// Return source paths for policies whose shader is outside the canonical
/// managed shader directory. SQLite is authoritative for both policy target
/// and shader source location.
pub fn external_policy_entries(
    _config_path: &Path,
    target: PolicyTarget,
) -> Result<Vec<(i64, String, String, PathBuf)>, String> {

    let managed_directory =
        crate::locate_paths::shader_dir();


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while reading external {} shader paths: {}",
                        target.name(),
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
                     s.filename,
                     s.source_path
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_target = ?1
                 ORDER BY lower(s.filename),
                          lower(p.policy_name),
                          p.policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare external {} shader-path query: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                rusqlite::params![
                    target.name()
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query external {} shader paths: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


    let mut paths =
        Vec::new();


    for row in rows {

        let (
            policy_id,
            policy_name,
            filename,
            source_directory,
        ) =
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode external {} shader path: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


        let source_path =
            PathBuf::from(
                source_directory
            )
            .join(
                &filename
            );


        let source_parent =
            source_path
                .parent()
                .unwrap_or_else(
                    || Path::new("")
                );


        if paths_refer_to_same_source(
            source_parent,
            &managed_directory,
        ) {
            continue;
        }


        paths.push(
            (
                policy_id,
                policy_name,
                filename,
                source_path,
            )
        );
    }


    Ok(
        paths
    )
}


pub fn external_policy_paths(
    _config_path: &Path,
    target: PolicyTarget,
) -> Result<Vec<(String, String, PathBuf)>, String> {

    let managed_directory =
        crate::locate_paths::shader_dir();


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while reading external {} shader paths: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT
                     p.policy_name,
                     s.filename,
                     s.source_path
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_target = ?1
                 ORDER BY lower(s.filename),
                          lower(p.policy_name),
                          p.policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare external {} shader-path query: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                rusqlite::params![
                    target.name()
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query external {} shader paths: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


    let mut paths =
        Vec::new();


    for row in rows {

        let (
            policy_name,
            filename,
            source_directory,
        ) =
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode external {} shader path: {}",
                        target.name(),
                        error,
                    )
                }
            )?;


        let source_path =
            PathBuf::from(
                source_directory
            )
            .join(
                &filename
            );


        let source_parent =
            source_path
                .parent()
                .unwrap_or_else(
                    || Path::new("")
                );


        if paths_refer_to_same_source(
            source_parent,
            &managed_directory,
        ) {
            continue;
        }


        paths.push(
            (
                policy_name,
                filename,
                source_path,
            )
        );
    }


    Ok(
        paths
    )
}


pub fn policy_source_path(
    _config_path: &Path,
    target: PolicyTarget,
    shader: &str,
) -> Result<Option<PathBuf>, String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while reading {} source path for '{}': {}",
                        target.name(),
                        shader,
                        error,
                    )
                }
            )?;


    let row =
        connection
            .query_row(
                "SELECT
                     s.filename,
                     s.source_path
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE p.policy_target = ?1
                   AND lower(s.filename) = lower(?2)
                 ORDER BY p.policy_id
                 LIMIT 1",
                rusqlite::params![
                    target.name(),
                    shader,
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                        )
                    )
                },
            );


    match row {
        Ok((
            filename,
            source_directory,
        )) => {
            Ok(
                Some(
                    PathBuf::from(
                        source_directory
                    )
                    .join(
                        filename
                    )
                )
            )
        }

        Err(
            rusqlite::Error::QueryReturnedNoRows
        ) => {
            Ok(
                None
            )
        }

        Err(error) => {
            Err(
                format!(
                    "Unable to query {} source path for '{}': {}",
                    target.name(),
                    shader,
                    error,
                )
            )
        }
    }
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
    target: PolicyTarget,
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
                     WHERE policy_name_key = ?1
                       AND policy_target = ?2",
                    rusqlite::params![
                        key,
                        target.name(),
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
            "Unable to generate an available suggested Policy Name for shader '{}'",
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


pub fn policy_display_name_from_key(
    key: &str,
) -> &str {

    // SQLite Policy Names are already user-facing identifiers. The old TOML
    // storage-key hash suffix no longer exists.
    key
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
