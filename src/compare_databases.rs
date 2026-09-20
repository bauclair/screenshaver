use rusqlite::{
    Connection,
    OpenFlags,
    types::ValueRef,
};

use std::collections::{
    BTreeMap,
    BTreeSet,
};

use std::path::{
    Path,
    PathBuf,
};


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub struct ComparisonSummary {
    pub differences: usize,
}


#[derive(
    Debug,
    Clone,
)]
struct SchemaObject {
    object_type: String,
    name: String,
    table_name: String,
    sql: Option<String>,
}


#[derive(
    Debug,
    Clone,
)]
struct ColumnInfo {
    cid: i64,
    name: String,
    declared_type: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key_position: i64,
}


#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
enum CellValue {
    Null,
    Integer(i64),
    Real(u64),
    Text(String),
    Blob(Vec<u8>),
}


impl CellValue {

    fn display(&self) -> String {

        match self {

            Self::Null => {
                "NULL".to_string()
            }

            Self::Integer(value) => {
                value.to_string()
            }

            Self::Real(bits) => {
                f64::from_bits(*bits)
                    .to_string()
            }

            Self::Text(value) => {
                format!(
                    "\"{}\"",
                    escape_text(value),
                )
            }

            Self::Blob(value) => {
                format!(
                    "BLOB[{}] 0x{}",
                    value.len(),
                    bytes_to_hex(value),
                )
            }
        }
    }
}


type RowData =
    Vec<CellValue>;



pub fn compare(
    database_a: &str,
    database_b: &str,
    exclude_metadata: bool,
    exclude_local_config: bool,
) -> Result<ComparisonSummary, String> {

    if !exclude_metadata {
        return compare_literal_internal(
            database_a,
            database_b,
            exclude_local_config,
        );
    }

    let path_a =
        validate_database_path(
            database_a,
            "Database A",
        )?;

    let path_b =
        validate_database_path(
            database_b,
            "Database B",
        )?;

    println!("Screenshaver database comparison");
    println!("Database A: {}", path_a.display());
    println!("Database B: {}", path_b.display());
    if exclude_local_config {
        println!("Comparison mode: semantic (--exclude-metadata --exclude-local-config)");
    } else {
        println!("Comparison mode: semantic (--exclude-metadata)");
    }
    println!();

    let connection_a =
        open_read_only(
            &path_a,
            "Database A",
        )?;

    let connection_b =
        open_read_only(
            &path_b,
            "Database B",
        )?;

    verify_database(
        &connection_a,
        "Database A",
    )?;

    verify_database(
        &connection_b,
        "Database B",
    )?;

    let mut difference_count = 0usize;

    println!("Metadata excluded:");
    println!("  local shader, policy, playlist, and relationship IDs");
    println!("  creation/modification/addition timestamps");
    println!("  schema/application provenance metadata");
    println!("  derived shader validation/runtime-package metadata");
    println!("Semantic relationships are still compared by names, targets, shader paths, and playlist order.");
    if exclude_local_config {
        println!("Local configuration excluded:");
        println!("  runtime target selections/modes/intervals");
        println!("  application defaults");
        println!("  screensaver/wallpaper target defaults");
    }
    println!();

    compare_semantic_query(
        &connection_a,
        &connection_b,
        "shaders",
        "SELECT filename, source_path, shader_type, source_hash
           FROM shaders
          WHERE lower(trim(filename)) <> 'default.glsl'
          ORDER BY source_path, filename",
        &["filename", "source_path", "shader_type", "source_hash"],
        &["source_path", "filename"],
        &mut difference_count,
    )?;

    compare_semantic_query(
        &connection_a,
        &connection_b,
        "shader_policies",
        "SELECT p.policy_name,
                p.policy_name_key,
                p.policy_target,
                s.filename AS shader_filename,
                s.source_path AS shader_source_path,
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
           FROM shader_policies p
           JOIN shaders s ON s.shader_id = p.shader_id
          WHERE lower(trim(s.filename)) <> 'default.glsl'
          ORDER BY p.policy_name_key, p.policy_target, s.source_path, s.filename",
        &[
            "policy_name", "policy_name_key", "policy_target",
            "shader_filename", "shader_source_path",
            "texture_mode", "texture_family", "texture_primitives",
            "palette_mode", "palette_color", "rendered_fps",
            "animation_speed", "starting_offset", "anti_aliasing",
            "dithering", "color_precision", "render_scale",
            "audiovisual_effect", "bloom_intensity", "bloom_saturation",
            "bloom_threshold", "bloom_frequency_rotation",
            "bloom_frequency_invert", "invert_colors",
            "flip_horizontal", "flip_vertical", "hue_rotation",
        ],
        &[
            "policy_name_key", "policy_target",
            "shader_source_path", "shader_filename",
        ],
        &mut difference_count,
    )?;

    compare_semantic_query(
        &connection_a,
        &connection_b,
        "playlists",
        "SELECT playlist_name, playlist_name_key, description
           FROM playlists
          ORDER BY playlist_name_key",
        &["playlist_name", "playlist_name_key", "description"],
        &["playlist_name_key"],
        &mut difference_count,
    )?;

    compare_semantic_query(
        &connection_a,
        &connection_b,
        "playlist_members",
        "SELECT pl.playlist_name_key,
                p.policy_name_key,
                p.policy_target,
                s.filename AS shader_filename,
                s.source_path AS shader_source_path,
                pm.position
           FROM playlist_members pm
           JOIN playlists pl ON pl.playlist_id = pm.playlist_id
           JOIN shader_policies p ON p.policy_id = pm.policy_id
           JOIN shaders s ON s.shader_id = p.shader_id
          WHERE lower(trim(s.filename)) <> 'default.glsl'
          ORDER BY pl.playlist_name_key, pm.position",
        &[
            "playlist_name_key", "policy_name_key", "policy_target",
            "shader_filename", "shader_source_path", "position",
        ],
        &[
            "playlist_name_key", "position",
            "policy_name_key", "policy_target",
            "shader_source_path", "shader_filename",
        ],
        &mut difference_count,
    )?;

    if !exclude_local_config {
        compare_semantic_query(
            &connection_a,
            &connection_b,
            "runtime_targets",
            "SELECT rt.target,
                    rt.display_mode,
                    rt.interval_seconds,
                    p.policy_name_key AS single_policy_name_key,
                    p.policy_target AS single_policy_target,
                    s.filename AS single_shader_filename,
                    s.source_path AS single_shader_source_path,
                    pl.playlist_name_key
               FROM runtime_targets rt
               LEFT JOIN shader_policies p ON p.policy_id = rt.single_policy_id
               LEFT JOIN shaders s ON s.shader_id = p.shader_id
               LEFT JOIN playlists pl ON pl.playlist_id = rt.playlist_id
              ORDER BY rt.target",
            &[
                "target", "display_mode", "interval_seconds",
                "single_policy_name_key", "single_policy_target",
                "single_shader_filename", "single_shader_source_path",
                "playlist_name_key",
            ],
            &["target"],
            &mut difference_count,
        )?;
    }

    for table_name in [
        "app_defaults",
        "target_defaults",
        "textures",
        "curated_palette",
    ] {
        if exclude_local_config
            && matches!(
                table_name,
                "app_defaults" | "target_defaults"
            )
        {
            continue;
        }

        compare_table(
            &connection_a,
            &connection_b,
            table_name,
            &mut difference_count,
        )?;
    }

    Ok(
        ComparisonSummary {
            differences: difference_count,
        }
    )
}


fn compare_semantic_query(
    connection_a: &Connection,
    connection_b: &Connection,
    label: &str,
    sql: &str,
    columns: &[&str],
    key_columns: &[&str],
    difference_count: &mut usize,
) -> Result<(), String> {

    println!("== Semantic table: {} ==", label);

    let rows_a =
        load_semantic_rows(
            connection_a,
            sql,
            columns,
            key_columns,
        )?;

    let rows_b =
        load_semantic_rows(
            connection_b,
            sql,
            columns,
            key_columns,
        )?;

    if rows_a.len() != rows_b.len() {
        report_difference(
            difference_count,
            &format!("{} semantic row count", label),
            &rows_a.len().to_string(),
            &rows_b.len().to_string(),
        );
    }

    let keys =
        rows_a.keys()
            .chain(rows_b.keys())
            .cloned()
            .collect::<BTreeSet<_>>();

    for key in keys {
        match (
            rows_a.get(&key),
            rows_b.get(&key),
        ) {
            (Some(a), Some(b)) => {
                for column in columns {
                    let av = a.get(*column);
                    let bv = b.get(*column);

                    if av != bv {
                        report_difference(
                            difference_count,
                            &format!(
                                "{} row [{}] column {}",
                                label,
                                key,
                                column,
                            ),
                            &av.map(CellValue::display)
                                .unwrap_or_else(|| "<missing>".to_string()),
                            &bv.map(CellValue::display)
                                .unwrap_or_else(|| "<missing>".to_string()),
                        );
                    }
                }
            }

            (Some(a), None) => {
                *difference_count += 1;
                println!(
                    "DIFFERENCE {}: {} semantic row [{}] exists only in Database A",
                    *difference_count,
                    label,
                    key,
                );
                println!("  A: {}", display_semantic_row(a));
                println!("  B: <missing>");
            }

            (None, Some(b)) => {
                *difference_count += 1;
                println!(
                    "DIFFERENCE {}: {} semantic row [{}] exists only in Database B",
                    *difference_count,
                    label,
                    key,
                );
                println!("  A: <missing>");
                println!("  B: {}", display_semantic_row(b));
            }

            (None, None) => {}
        }
    }

    println!();
    Ok(())
}


fn load_semantic_rows(
    connection: &Connection,
    sql: &str,
    columns: &[&str],
    key_columns: &[&str],
) -> Result<BTreeMap<String, BTreeMap<String, CellValue>>, String> {

    let mut statement =
        connection.prepare(sql)
            .map_err(|error| {
                format!(
                    "Unable to prepare semantic database comparison: {}",
                    error
                )
            })?;

    let rows =
        statement.query_map(
            [],
            |row| {
                let mut values = BTreeMap::new();

                for (index, column) in columns.iter().enumerate() {
                    values.insert(
                        (*column).to_string(),
                        cell_from_value_ref(
                            row.get_ref(index)?
                        ),
                    );
                }

                Ok(values)
            },
        )
        .map_err(|error| {
            format!(
                "Unable to execute semantic database comparison: {}",
                error
            )
        })?;

    let mut output = BTreeMap::new();

    for row in rows {
        let values =
            row.map_err(|error| error.to_string())?;

        let base_key =
            key_columns.iter()
                .map(|column| {
                    format!(
                        "{}={}",
                        column,
                        values.get(*column)
                            .map(CellValue::display)
                            .unwrap_or_else(|| "<missing>".to_string()),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

        let mut key = base_key.clone();
        let mut occurrence = 2usize;

        while output.contains_key(&key) {
            key = format!(
                "{} occurrence={}",
                base_key,
                occurrence,
            );
            occurrence += 1;
        }

        output.insert(
            key,
            values,
        );
    }

    Ok(output)
}


fn display_semantic_row(
    row: &BTreeMap<String, CellValue>,
) -> String {

    row.iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                name,
                value.display(),
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}


fn compare_literal_internal(
    database_a: &str,
    database_b: &str,
    exclude_local_config: bool,
) -> Result<ComparisonSummary, String> {

    let path_a =
        validate_database_path(
            database_a,
            "Database A",
        )?;

    let path_b =
        validate_database_path(
            database_b,
            "Database B",
        )?;


    println!(
        "Screenshaver database comparison"
    );

    println!(
        "Database A: {}",
        path_a.display(),
    );

    println!(
        "Database B: {}",
        path_b.display(),
    );

    if exclude_local_config {
        println!(
            "Comparison mode: literal (--exclude-local-config)"
        );
        println!();
        println!("Local configuration excluded:");
        println!("  runtime target selections/modes/intervals");
        println!("  application defaults");
        println!("  screensaver/wallpaper target defaults");
    } else {
        println!(
            "Comparison mode: literal"
        );
    }

    println!();


    let connection_a =
        open_read_only(
            &path_a,
            "Database A",
        )?;

    let connection_b =
        open_read_only(
            &path_b,
            "Database B",
        )?;


    verify_database(
        &connection_a,
        "Database A",
    )?;

    verify_database(
        &connection_b,
        "Database B",
    )?;


    let mut difference_count =
        0usize;


    compare_database_pragmas(
        &connection_a,
        &connection_b,
        &mut difference_count,
    )?;


    let schema_a =
        load_schema(
            &connection_a
        )?;

    let schema_b =
        load_schema(
            &connection_b
        )?;


    compare_schema(
        &schema_a,
        &schema_b,
        &mut difference_count,
    );


    let tables_a =
        schema_a
            .values()
            .filter(
                |object| {
                    object.object_type
                        == "table"
                        && !object.name.starts_with(
                            "sqlite_"
                        )
                }
            )
            .map(
                |object| {
                    object.name.clone()
                }
            )
            .collect::<BTreeSet<_>>();

    let tables_b =
        schema_b
            .values()
            .filter(
                |object| {
                    object.object_type
                        == "table"
                        && !object.name.starts_with(
                            "sqlite_"
                        )
                }
            )
            .map(
                |object| {
                    object.name.clone()
                }
            )
            .collect::<BTreeSet<_>>();


    for table_name in
        tables_a.intersection(
            &tables_b
        )
    {
        if exclude_local_config
            && matches!(
                table_name.as_str(),
                "runtime_targets"
                    | "app_defaults"
                    | "target_defaults"
            )
        {
            continue;
        }

        compare_table(
            &connection_a,
            &connection_b,
            table_name,
            &mut difference_count,
        )?;
    }


    Ok(
        ComparisonSummary {
            differences:
                difference_count,
        }
    )
}


fn validate_database_path(
    value: &str,
    label: &str,
) -> Result<PathBuf, String> {

    let path =
        PathBuf::from(
            value
        );


    if !path.exists() {
        return Err(
            format!(
                "{} does not exist: {}",
                label,
                path.display(),
            )
        );
    }


    if !path.is_file() {
        return Err(
            format!(
                "{} is not a regular file: {}",
                label,
                path.display(),
            )
        );
    }


    Ok(
        path
    )
}


fn open_read_only(
    path: &Path,
    label: &str,
) -> Result<Connection, String> {

    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(
        |error| {
            format!(
                "Unable to open {} read-only ({}): {}",
                label,
                path.display(),
                error,
            )
        }
    )
}


fn verify_database(
    connection: &Connection,
    label: &str,
) -> Result<(), String> {

    let integrity:
        String =
        connection
            .query_row(
                "PRAGMA integrity_check;",
                [],
                |row| row.get(0),
            )
            .map_err(
                |error| {
                    format!(
                        "{} integrity check failed to execute: {}",
                        label,
                        error,
                    )
                }
            )?;


    if integrity
        .trim()
        .eq_ignore_ascii_case(
            "ok"
        )
    {
        Ok(())
    } else {
        Err(
            format!(
                "{} failed SQLite integrity_check: {}",
                label,
                integrity,
            )
        )
    }
}


fn compare_database_pragmas(
    connection_a: &Connection,
    connection_b: &Connection,
    difference_count: &mut usize,
) -> Result<(), String> {

    println!(
        "== Database properties =="
    );


    let pragmas = [
        "application_id",
        "auto_vacuum",
        "encoding",
        "page_size",
        "schema_version",
        "user_version",
    ];


    for pragma in pragmas {

        let value_a =
            pragma_value(
                connection_a,
                pragma,
            )?;

        let value_b =
            pragma_value(
                connection_b,
                pragma,
            )?;


        if value_a != value_b {
            report_difference(
                difference_count,
                &format!(
                    "PRAGMA {}",
                    pragma,
                ),
                &value_a,
                &value_b,
            );
        }
    }


    println!();

    Ok(())
}


fn pragma_value(
    connection: &Connection,
    pragma: &str,
) -> Result<String, String> {

    let sql =
        format!(
            "PRAGMA {};",
            pragma,
        );


    connection
        .query_row(
            &sql,
            [],
            |row| {
                let value =
                    row.get_ref(0)?;

                Ok(
                    cell_from_value_ref(
                        value
                    )
                    .display()
                )
            },
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read {}: {}",
                    sql,
                    error,
                )
            }
        )
}


fn load_schema(
    connection: &Connection,
) -> Result<
    BTreeMap<String, SchemaObject>,
    String,
> {

    let mut statement =
        connection
            .prepare(
                "SELECT type, name, tbl_name, sql
                   FROM sqlite_schema
                  WHERE name NOT LIKE 'sqlite_%'
                  ORDER BY type, name;"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read SQLite schema: {}",
                        error,
                    )
                }
            )?;


    let mut rows =
        statement
            .query(
                []
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query SQLite schema: {}",
                        error,
                    )
                }
            )?;


    let mut objects =
        BTreeMap::new();


    while let Some(row) =
        rows.next()
            .map_err(
                |error| {
                    format!(
                        "Unable to iterate SQLite schema: {}",
                        error,
                    )
                }
            )?
    {
        let object =
            SchemaObject {
                object_type:
                    row.get(0)
                        .map_err(
                            |error| error.to_string()
                        )?,

                name:
                    row.get(1)
                        .map_err(
                            |error| error.to_string()
                        )?,

                table_name:
                    row.get(2)
                        .map_err(
                            |error| error.to_string()
                        )?,

                sql:
                    row.get(3)
                        .map_err(
                            |error| error.to_string()
                        )?,
            };


        let key =
            format!(
                "{}\0{}",
                object.object_type,
                object.name,
            );


        objects.insert(
            key,
            object,
        );
    }


    Ok(
        objects
    )
}


fn compare_schema(
    schema_a: &BTreeMap<String, SchemaObject>,
    schema_b: &BTreeMap<String, SchemaObject>,
    difference_count: &mut usize,
) {

    println!(
        "== Schema objects =="
    );


    let keys =
        schema_a
            .keys()
            .chain(
                schema_b.keys()
            )
            .cloned()
            .collect::<BTreeSet<_>>();


    for key in keys {

        match (
            schema_a.get(
                &key
            ),
            schema_b.get(
                &key
            ),
        ) {

            (
                Some(object_a),
                Some(object_b),
            ) => {

                if object_a.table_name
                    != object_b.table_name
                {
                    report_difference(
                        difference_count,
                        &format!(
                            "{} {} table association",
                            object_a.object_type,
                            object_a.name,
                        ),
                        &object_a.table_name,
                        &object_b.table_name,
                    );
                }


                let sql_a =
                    normalize_schema_sql(
                        object_a.sql.as_deref()
                    );

                let sql_b =
                    normalize_schema_sql(
                        object_b.sql.as_deref()
                    );


                if sql_a != sql_b {
                    report_difference(
                        difference_count,
                        &format!(
                            "{} {} definition",
                            object_a.object_type,
                            object_a.name,
                        ),
                        &sql_a,
                        &sql_b,
                    );
                }
            }

            (
                Some(object),
                None,
            ) => {
                report_presence_difference(
                    difference_count,
                    &format!(
                        "{} {}",
                        object.object_type,
                        object.name,
                    ),
                    true,
                    false,
                );
            }

            (
                None,
                Some(object),
            ) => {
                report_presence_difference(
                    difference_count,
                    &format!(
                        "{} {}",
                        object.object_type,
                        object.name,
                    ),
                    false,
                    true,
                );
            }

            (
                None,
                None,
            ) => {}
        }
    }


    println!();
}


fn compare_table(
    connection_a: &Connection,
    connection_b: &Connection,
    table_name: &str,
    difference_count: &mut usize,
) -> Result<(), String> {

    println!(
        "== Table: {} ==",
        table_name,
    );


    let columns_a =
        load_columns(
            connection_a,
            table_name,
        )?;

    let columns_b =
        load_columns(
            connection_b,
            table_name,
        )?;


    compare_columns(
        table_name,
        &columns_a,
        &columns_b,
        difference_count,
    );


    let names_a =
        columns_a
            .iter()
            .map(
                |column| {
                    column.name.clone()
                }
            )
            .collect::<Vec<_>>();

    let names_b =
        columns_b
            .iter()
            .map(
                |column| {
                    column.name.clone()
                }
            )
            .collect::<Vec<_>>();


    if names_a != names_b {
        println!(
            "Row comparison skipped because the column layouts differ."
        );

        println!();

        return Ok(());
    }


    let rows_a =
        load_rows(
            connection_a,
            table_name,
            &columns_a,
        )?;

    let rows_b =
        load_rows(
            connection_b,
            table_name,
            &columns_b,
        )?;


    if rows_a.len()
        != rows_b.len()
    {
        report_difference(
            difference_count,
            &format!(
                "{} row count",
                table_name,
            ),
            &rows_a.len().to_string(),
            &rows_b.len().to_string(),
        );
    }


    let primary_key_indexes =
        primary_key_indexes(
            &columns_a
        );


    if !primary_key_indexes.is_empty() {

        compare_rows_by_key(
            table_name,
            &columns_a,
            &primary_key_indexes,
            &rows_a,
            &rows_b,
            difference_count,
        );

    } else {

        compare_rows_as_multiset(
            table_name,
            &columns_a,
            &rows_a,
            &rows_b,
            difference_count,
        );
    }


    println!();

    Ok(())
}


fn load_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<Vec<ColumnInfo>, String> {

    let sql =
        format!(
            "PRAGMA table_info({});",
            quote_identifier(
                table_name
            ),
        );


    let mut statement =
        connection
            .prepare(
                &sql
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to inspect table '{}': {}",
                        table_name,
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
                        ColumnInfo {
                            cid:
                                row.get(0)?,

                            name:
                                row.get(1)?,

                            declared_type:
                                row.get(2)?,

                            not_null:
                                row.get::<_, i64>(3)?
                                    != 0,

                            default_value:
                                row.get(4)?,

                            primary_key_position:
                                row.get(5)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read columns for '{}': {}",
                        table_name,
                        error,
                    )
                }
            )?;


    rows.collect::<Result<Vec<_>, _>>()
        .map_err(
            |error| {
                format!(
                    "Unable to collect columns for '{}': {}",
                    table_name,
                    error,
                )
            }
        )
}


fn compare_columns(
    table_name: &str,
    columns_a: &[ColumnInfo],
    columns_b: &[ColumnInfo],
    difference_count: &mut usize,
) {

    let map_a =
        columns_a
            .iter()
            .map(
                |column| {
                    (
                        column.name.clone(),
                        column,
                    )
                }
            )
            .collect::<BTreeMap<_, _>>();

    let map_b =
        columns_b
            .iter()
            .map(
                |column| {
                    (
                        column.name.clone(),
                        column,
                    )
                }
            )
            .collect::<BTreeMap<_, _>>();

    let names =
        map_a
            .keys()
            .chain(
                map_b.keys()
            )
            .cloned()
            .collect::<BTreeSet<_>>();


    for name in names {

        match (
            map_a.get(
                &name
            ),
            map_b.get(
                &name
            ),
        ) {

            (
                Some(a),
                Some(b),
            ) => {

                let definition_a =
                    format!(
                        "cid={} type={} not_null={} default={:?} pk={}",
                        a.cid,
                        a.declared_type,
                        a.not_null,
                        a.default_value,
                        a.primary_key_position,
                    );

                let definition_b =
                    format!(
                        "cid={} type={} not_null={} default={:?} pk={}",
                        b.cid,
                        b.declared_type,
                        b.not_null,
                        b.default_value,
                        b.primary_key_position,
                    );


                if definition_a
                    != definition_b
                {
                    report_difference(
                        difference_count,
                        &format!(
                            "{} column {}",
                            table_name,
                            name,
                        ),
                        &definition_a,
                        &definition_b,
                    );
                }
            }

            (
                Some(_),
                None,
            ) => {
                report_presence_difference(
                    difference_count,
                    &format!(
                        "{} column {}",
                        table_name,
                        name,
                    ),
                    true,
                    false,
                );
            }

            (
                None,
                Some(_),
            ) => {
                report_presence_difference(
                    difference_count,
                    &format!(
                        "{} column {}",
                        table_name,
                        name,
                    ),
                    false,
                    true,
                );
            }

            (
                None,
                None,
            ) => {}
        }
    }
}


fn load_rows(
    connection: &Connection,
    table_name: &str,
    columns: &[ColumnInfo],
) -> Result<Vec<RowData>, String> {

    let column_list =
        columns
            .iter()
            .map(
                |column| {
                    quote_identifier(
                        &column.name
                    )
                }
            )
            .collect::<Vec<_>>()
            .join(
                ", "
            );


    let sql =
        format!(
            "SELECT {} FROM {}",
            column_list,
            quote_identifier(
                table_name
            ),
        );


    let mut statement =
        connection
            .prepare(
                &sql
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare row comparison for '{}': {}",
                        table_name,
                        error,
                    )
                }
            )?;


    let column_count =
        columns.len();


    let rows =
        statement
            .query_map(
                [],
                move |row| {

                    let mut values =
                        Vec::with_capacity(
                            column_count
                        );


                    for index in
                        0..column_count
                    {
                        values.push(
                            cell_from_value_ref(
                                row.get_ref(
                                    index
                                )?
                            )
                        );
                    }


                    Ok(
                        values
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query rows from '{}': {}",
                        table_name,
                        error,
                    )
                }
            )?;


    rows.collect::<Result<Vec<_>, _>>()
        .map_err(
            |error| {
                format!(
                    "Unable to collect rows from '{}': {}",
                    table_name,
                    error,
                )
            }
        )
}


fn primary_key_indexes(
    columns: &[ColumnInfo],
) -> Vec<usize> {

    let mut indexed =
        columns
            .iter()
            .enumerate()
            .filter(
                |(_, column)| {
                    column.primary_key_position
                        > 0
                }
            )
            .map(
                |(index, column)| {
                    (
                        column.primary_key_position,
                        index,
                    )
                }
            )
            .collect::<Vec<_>>();


    indexed.sort_by_key(
        |(position, _)| {
            *position
        }
    );


    indexed
        .into_iter()
        .map(
            |(_, index)| {
                index
            }
        )
        .collect()
}


fn compare_rows_by_key(
    table_name: &str,
    columns: &[ColumnInfo],
    primary_key_indexes: &[usize],
    rows_a: &[RowData],
    rows_b: &[RowData],
    difference_count: &mut usize,
) {

    let map_a =
        keyed_rows(
            rows_a,
            primary_key_indexes,
        );

    let map_b =
        keyed_rows(
            rows_b,
            primary_key_indexes,
        );

    let keys =
        map_a
            .keys()
            .chain(
                map_b.keys()
            )
            .cloned()
            .collect::<BTreeSet<_>>();


    for key in keys {

        match (
            map_a.get(
                &key
            ),
            map_b.get(
                &key
            ),
        ) {

            (
                Some(row_a),
                Some(row_b),
            ) => {

                for (
                    index,
                    column,
                ) in columns
                    .iter()
                    .enumerate()
                {
                    if row_a[index]
                        != row_b[index]
                    {
                        report_difference(
                            difference_count,
                            &format!(
                                "{} row [{}] column {}",
                                table_name,
                                display_key(
                                    columns,
                                    primary_key_indexes,
                                    row_a,
                                ),
                                column.name,
                            ),
                            &row_a[index].display(),
                            &row_b[index].display(),
                        );
                    }
                }
            }

            (
                Some(row),
                None,
            ) => {
                *difference_count +=
                    1;

                println!(
                    "DIFFERENCE {}: {} row [{}] exists only in Database A",
                    *difference_count,
                    table_name,
                    display_key(
                        columns,
                        primary_key_indexes,
                        row,
                    ),
                );

                println!(
                    "  A: {}",
                    display_row(
                        columns,
                        row,
                    ),
                );

                println!(
                    "  B: <missing>"
                );
            }

            (
                None,
                Some(row),
            ) => {
                *difference_count +=
                    1;

                println!(
                    "DIFFERENCE {}: {} row [{}] exists only in Database B",
                    *difference_count,
                    table_name,
                    display_key(
                        columns,
                        primary_key_indexes,
                        row,
                    ),
                );

                println!(
                    "  A: <missing>"
                );

                println!(
                    "  B: {}",
                    display_row(
                        columns,
                        row,
                    ),
                );
            }

            (
                None,
                None,
            ) => {}
        }
    }
}


fn keyed_rows<'a>(
    rows: &'a [RowData],
    primary_key_indexes: &[usize],
) -> BTreeMap<Vec<CellValue>, &'a RowData> {

    rows.iter()
        .map(
            |row| {

                let key =
                    primary_key_indexes
                        .iter()
                        .map(
                            |index| {
                                row[*index]
                                    .clone()
                            }
                        )
                        .collect::<Vec<_>>();


                (
                    key,
                    row,
                )
            }
        )
        .collect()
}


fn compare_rows_as_multiset(
    table_name: &str,
    columns: &[ColumnInfo],
    rows_a: &[RowData],
    rows_b: &[RowData],
    difference_count: &mut usize,
) {

    let counts_a =
        row_counts(
            rows_a
        );

    let counts_b =
        row_counts(
            rows_b
        );

    let rows =
        counts_a
            .keys()
            .chain(
                counts_b.keys()
            )
            .cloned()
            .collect::<BTreeSet<_>>();


    for row in rows {

        let count_a =
            counts_a
                .get(
                    &row
                )
                .copied()
                .unwrap_or(0);

        let count_b =
            counts_b
                .get(
                    &row
                )
                .copied()
                .unwrap_or(0);


        if count_a != count_b {

            *difference_count +=
                1;

            println!(
                "DIFFERENCE {}: {} row occurrence count differs",
                *difference_count,
                table_name,
            );

            println!(
                "  Row: {}",
                display_row(
                    columns,
                    &row,
                ),
            );

            println!(
                "  A: {} occurrence(s)",
                count_a,
            );

            println!(
                "  B: {} occurrence(s)",
                count_b,
            );
        }
    }
}


fn row_counts(
    rows: &[RowData],
) -> BTreeMap<RowData, usize> {

    let mut counts =
        BTreeMap::new();


    for row in rows {

        *counts
            .entry(
                row.clone()
            )
            .or_insert(
                0
            ) += 1;
    }


    counts
}


fn display_key(
    columns: &[ColumnInfo],
    primary_key_indexes: &[usize],
    row: &RowData,
) -> String {

    primary_key_indexes
        .iter()
        .map(
            |index| {
                format!(
                    "{}={}",
                    columns[*index].name,
                    row[*index].display(),
                )
            }
        )
        .collect::<Vec<_>>()
        .join(
            ", "
        )
}


fn display_row(
    columns: &[ColumnInfo],
    row: &RowData,
) -> String {

    columns
        .iter()
        .enumerate()
        .map(
            |(index, column)| {
                format!(
                    "{}={}",
                    column.name,
                    row[index].display(),
                )
            }
        )
        .collect::<Vec<_>>()
        .join(
            ", "
        )
}


fn report_difference(
    difference_count: &mut usize,
    subject: &str,
    value_a: &str,
    value_b: &str,
) {

    *difference_count +=
        1;

    println!(
        "DIFFERENCE {}: {}",
        *difference_count,
        subject,
    );

    println!(
        "  A: {}",
        value_a,
    );

    println!(
        "  B: {}",
        value_b,
    );
}


fn report_presence_difference(
    difference_count: &mut usize,
    subject: &str,
    present_a: bool,
    present_b: bool,
) {

    report_difference(
        difference_count,
        subject,
        if present_a {
            "<present>"
        } else {
            "<missing>"
        },
        if present_b {
            "<present>"
        } else {
            "<missing>"
        },
    );
}


fn cell_from_value_ref(
    value: ValueRef<'_>,
) -> CellValue {

    match value {

        ValueRef::Null => {
            CellValue::Null
        }

        ValueRef::Integer(value) => {
            CellValue::Integer(
                value
            )
        }

        ValueRef::Real(value) => {
            CellValue::Real(
                value.to_bits()
            )
        }

        ValueRef::Text(value) => {
            CellValue::Text(
                String::from_utf8_lossy(
                    value
                )
                .into_owned()
            )
        }

        ValueRef::Blob(value) => {
            CellValue::Blob(
                value.to_vec()
            )
        }
    }
}


fn quote_identifier(
    value: &str,
) -> String {

    format!(
        "\"{}\"",
        value.replace(
            '"',
            "\"\"",
        ),
    )
}


fn normalize_schema_sql(
    value: Option<&str>,
) -> String {

    value
        .unwrap_or(
            "<none>"
        )
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(
            " "
        )
}


fn escape_text(
    value: &str,
) -> String {

    value
        .chars()
        .flat_map(
            |character| {

                match character {

                    '\\' => {
                        "\\\\".chars()
                            .collect::<Vec<_>>()
                    }

                    '"' => {
                        "\\\"".chars()
                            .collect::<Vec<_>>()
                    }

                    '\n' => {
                        "\\n".chars()
                            .collect::<Vec<_>>()
                    }

                    '\r' => {
                        "\\r".chars()
                            .collect::<Vec<_>>()
                    }

                    '\t' => {
                        "\\t".chars()
                            .collect::<Vec<_>>()
                    }

                    other => {
                        vec![
                            other
                        ]
                    }
                }
            }
        )
        .collect()
}


fn bytes_to_hex(
    bytes: &[u8],
) -> String {

    const HEX:
        &[u8; 16] =
        b"0123456789abcdef";


    let mut output =
        String::with_capacity(
            bytes.len() * 2
        );


    for byte in bytes {

        output.push(
            HEX[
                (byte >> 4)
                    as usize
            ] as char
        );

        output.push(
            HEX[
                (byte & 0x0f)
                    as usize
            ] as char
        );
    }


    output
}
