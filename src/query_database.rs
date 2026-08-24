//! Read-only database queries used by Policy List Query By Example.
//!
//! This module is the database-access boundary for QBE.  parse_qbe.rs owns
//! query semantics and SQL construction; qbe_layout.rs owns presentation.
//!
//! First checkpoint:
//! - load live texture names from the textures catalog,
//! - load live curated palette entries,
//! - load shader types currently represented in the shader inventory.
//!
//! Policy List filtering will be added in the next checkpoint.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QbePaletteChoice {
    pub color_hex: String,
    pub description: String,
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QbeLookupValues {
    pub texture_names: Vec<String>,
    pub palette_choices: Vec<QbePaletteChoice>,
    pub shader_types: Vec<String>,
}


pub fn load_qbe_lookup_values(
) -> Result<QbeLookupValues, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading QBE lookup values: {}",
                        error,
                    )
                }
            )?;


    Ok(
        QbeLookupValues {
            texture_names:
                load_texture_names_from_connection(
                    &connection
                )?,

            palette_choices:
                load_palette_choices_from_connection(
                    &connection
                )?,

            shader_types:
                load_shader_types_from_connection(
                    &connection
                )?,
        }
    )
}


pub fn load_texture_names(
) -> Result<Vec<String>, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading QBE texture names: {}",
                        error,
                    )
                }
            )?;


    load_texture_names_from_connection(
        &connection
    )
}


pub fn load_palette_choices(
) -> Result<Vec<QbePaletteChoice>, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading QBE palette choices: {}",
                        error,
                    )
                }
            )?;


    load_palette_choices_from_connection(
        &connection
    )
}


pub fn load_shader_types(
) -> Result<Vec<String>, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading QBE Shader Type values: {}",
                        error,
                    )
                }
            )?;


    load_shader_types_from_connection(
        &connection
    )
}


fn load_texture_names_from_connection(
    connection: &rusqlite::Connection,
) -> Result<Vec<String>, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT texture_name
                 FROM textures
                 ORDER BY lower(texture_name),
                          texture_name"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare QBE texture-catalog query: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [],
                |row| {
                    row.get::<_, String>(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query QBE texture choices: {}",
                        error,
                    )
                }
            )?;


    let mut values =
        Vec::new();


    for row in rows {
        values.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode QBE texture-catalog row: {}",
                        error,
                    )
                }
            )?
        );
    }


    Ok(
        values
    )
}


fn load_palette_choices_from_connection(
    connection: &rusqlite::Connection,
) -> Result<Vec<QbePaletteChoice>, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     color_hex,
                     description
                 FROM curated_palette
                 ORDER BY lower(description),
                          color_hex"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare QBE curated-palette query: {}",
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
                        QbePaletteChoice {
                            color_hex:
                                row.get(0)?,

                            description:
                                row.get(1)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query QBE curated-palette choices: {}",
                        error,
                    )
                }
            )?;


    let mut values =
        Vec::new();


    for row in rows {
        values.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode QBE curated-palette row: {}",
                        error,
                    )
                }
            )?
        );
    }


    Ok(
        values
    )
}


fn load_shader_types_from_connection(
    connection: &rusqlite::Connection,
) -> Result<Vec<String>, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT DISTINCT lower(shader_type)
                 FROM shaders
                 WHERE shader_type IS NOT NULL
                   AND trim(shader_type) <> ''
                   AND lower(shader_type) <> 'unknown'
                 ORDER BY lower(shader_type)"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare QBE Shader Type query: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [],
                |row| {
                    row.get::<_, String>(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query QBE Shader Type choices: {}",
                        error,
                    )
                }
            )?;


    let mut values =
        Vec::new();


    for row in rows {
        let stored =
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode QBE Shader Type row: {}",
                        error,
                    )
                }
            )?;


        let display =
            match stored.as_str() {
                "native" =>
                    "NativeGLSL".to_string(),

                "isf" =>
                    "ISF".to_string(),

                "shadertoy" =>
                    "ShaderToy".to_string(),

                _ =>
                    stored,
            };


        values.push(
            display
        );
    }


    Ok(
        values
    )
}

// ============================================================
// POLICY QBE EXECUTION
// ============================================================

#[derive(Clone, Debug, PartialEq)]
pub struct QbePolicyRecord {
    pub policy_id: i64,
    pub policy_name: String,
    pub policy_target: String,

    pub shader_id: i64,
    pub shader_filename: String,
    pub shader_source_path: String,
    pub shader_type: String,

    pub file_status: String,
    pub validation_status: String,
    pub validation_reason: Option<String>,
    pub validation_message: Option<String>,

    pub texture_mode: Option<String>,
    pub texture_family: Option<String>,
    pub texture_primitives: Option<i64>,

    pub palette_mode: Option<String>,
    pub palette_color: Option<String>,

    pub rendered_fps: Option<i64>,
    pub animation_speed: Option<f64>,

    pub anti_aliasing: Option<String>,
    pub dithering: Option<String>,
    pub color_precision: Option<String>,
    pub render_scale: Option<f64>,

    pub bloom_mode: Option<String>,
    pub bloom_intensity: Option<f64>,
    pub bloom_threshold: Option<f64>,

    pub invert_colors: Option<i64>,
    pub flip_horizontal: Option<i64>,
    pub flip_vertical: Option<i64>,
    pub hue_rotation: Option<f64>,
}


#[derive(Clone, Debug, Default, PartialEq)]
pub struct QbePolicyQueryResult {
    pub rows: Vec<QbePolicyRecord>,
    pub returned_count: usize,
    pub total_count: usize,
}


pub fn execute_policy_qbe(
    qbe: &crate::parse_qbe::QbeSql,
) -> Result<QbePolicyQueryResult, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while executing Policy List QBE: {}",
                        error,
                    )
                }
            )?;


    execute_policy_qbe_in_connection(
        &connection,
        qbe,
    )
}


pub fn execute_policy_qbe_state(
    state: &crate::parse_qbe::QbeState,
) -> Result<QbePolicyQueryResult, String> {

    let parsed =
        crate::parse_qbe::build_sql(
            state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to parse Policy List QBE: {}",
                    error.message(),
                )
            }
        )?;


    execute_policy_qbe(
        &parsed
    )
}


pub fn load_all_policy_records(
) -> Result<QbePolicyQueryResult, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading the complete Policy List: {}",
                        error,
                    )
                }
            )?;


    execute_policy_query_in_connection(
        &connection,
        "",
        &[],
    )
}


fn execute_policy_qbe_in_connection(
    connection: &rusqlite::Connection,
    qbe: &crate::parse_qbe::QbeSql,
) -> Result<QbePolicyQueryResult, String> {

    let bind_values =
        qbe.parameters
            .iter()
            .map(
                qbe_parameter_to_sqlite_value
            )
            .collect::<Vec<_>>();


    execute_policy_query_in_connection(
        connection,
        &qbe.where_clause,
        &bind_values,
    )
}


fn execute_policy_query_in_connection(
    connection: &rusqlite::Connection,
    where_clause: &str,
    bind_values: &[rusqlite::types::Value],
) -> Result<QbePolicyQueryResult, String> {

    let total_count_i64: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM shader_policies",
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count total shader policies for QBE: {}",
                        error,
                    )
                }
            )?;


    let sql =
        policy_select_sql(
            where_clause
        );


    let mut statement =
        connection
            .prepare(
                &sql
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Policy List QBE statement: {}",
                        error,
                    )
                }
            )?;


    let mapped =
        statement
            .query_map(
                rusqlite::params_from_iter(
                    bind_values.iter()
                ),
                |row| {
                    Ok(
                        QbePolicyRecord {
                            policy_id:
                                row.get(0)?,

                            policy_name:
                                row.get(1)?,

                            policy_target:
                                row.get(2)?,

                            shader_id:
                                row.get(3)?,

                            shader_filename:
                                row.get(4)?,

                            shader_source_path:
                                row.get(5)?,

                            shader_type:
                                row.get(6)?,

                            file_status:
                                row.get(7)?,

                            validation_status:
                                row.get(8)?,

                            validation_reason:
                                row.get(9)?,

                            validation_message:
                                row.get(10)?,

                            texture_mode:
                                row.get(11)?,

                            texture_family:
                                row.get(12)?,

                            texture_primitives:
                                row.get(13)?,

                            palette_mode:
                                row.get(14)?,

                            palette_color:
                                row.get(15)?,

                            rendered_fps:
                                row.get(16)?,

                            animation_speed:
                                row.get(17)?,

                            anti_aliasing:
                                row.get(18)?,

                            dithering:
                                row.get(19)?,

                            color_precision:
                                row.get(20)?,

                            render_scale:
                                row.get(21)?,

                            bloom_mode:
                                row.get(22)?,

                            bloom_intensity:
                                row.get(23)?,

                            bloom_threshold:
                                row.get(24)?,

                            invert_colors:
                                row.get(25)?,

                            flip_horizontal:
                                row.get(26)?,

                            flip_vertical:
                                row.get(27)?,

                            hue_rotation:
                                row.get(28)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to execute Policy List QBE: {}",
                        error,
                    )
                }
            )?;


    let mut rows =
        Vec::new();


    for mapped_row in mapped {
        rows.push(
            mapped_row
                .map_err(
                    |error| {
                        format!(
                            "Unable to decode Policy List QBE row: {}",
                            error,
                        )
                    }
                )?
        );
    }


    let total_count =
        usize::try_from(
            total_count_i64
        )
        .map_err(
            |_| {
                format!(
                    "Total shader-policy count {} cannot be represented as usize",
                    total_count_i64,
                )
            }
        )?;


    let returned_count =
        rows.len();


    Ok(
        QbePolicyQueryResult {
            rows,
            returned_count,
            total_count,
        }
    )
}


fn policy_select_sql(
    where_clause: &str,
) -> String {

    let mut sql =
        String::from(
            "SELECT
                 p.policy_id,
                 p.policy_name,
                 p.policy_target,

                 s.shader_id,
                 s.filename,
                 s.source_path,
                 s.shader_type,

                 s.file_status,
                 s.validation_status,
                 s.validation_reason,
                 s.validation_message,

                 p.texture_mode,
                 p.texture_family,
                 p.texture_primitives,

                 p.palette_mode,
                 p.palette_color,

                 p.rendered_fps,
                 p.animation_speed,

                 p.anti_aliasing,
                 p.dithering,
                 p.color_precision,
                 p.render_scale,

                 p.bloom_mode,
                 p.bloom_intensity,
                 p.bloom_threshold,

                 p.invert_colors,
                 p.flip_horizontal,
                 p.flip_vertical,
                 p.hue_rotation

             FROM shader_policies AS p
             JOIN shaders AS s
               ON s.shader_id = p.shader_id"
        );


    if !where_clause
        .trim()
        .is_empty()
    {
        sql.push_str(
            "\n             WHERE "
        );

        sql.push_str(
            where_clause
        );
    }


    // Keep database output deterministic. The Policy List remains free to
    // apply its current UI sort order after these records are adapted.
    sql.push_str(
        "\n             ORDER BY lower(p.policy_name),
                              lower(s.filename),
                              p.policy_id"
    );


    sql
}


fn qbe_parameter_to_sqlite_value(
    parameter: &crate::parse_qbe::QbeSqlParameter,
) -> rusqlite::types::Value {

    match parameter {
        crate::parse_qbe::QbeSqlParameter::Text(
            value
        ) => {
            rusqlite::types::Value::Text(
                value.clone()
            )
        }

        crate::parse_qbe::QbeSqlParameter::Integer(
            value
        ) => {
            rusqlite::types::Value::Integer(
                *value
            )
        }

        crate::parse_qbe::QbeSqlParameter::Real(
            value
        ) => {
            rusqlite::types::Value::Real(
                *value
            )
        }
    }
}

