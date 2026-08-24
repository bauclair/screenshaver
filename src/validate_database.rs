use rusqlite::{
        params,
        Connection,
};

const EXPECTED_SCHEMA_VERSION: i64 = 1;
const EXPECTED_RUNTIME_SOURCE_PREPARATION_VERSION: i64 = 1;


pub fn validate_integrity(
    connection: &Connection,
) -> Result<(), String> {

    let integrity_result: String =
        connection
            .query_row(
                "PRAGMA integrity_check",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to run SQLite integrity check: {}",
                        error,
                    )
                }
            )?;


    if integrity_result
        != "ok"
    {
        return Err(
            format!(
                "SQLite integrity check failed: {}",
                integrity_result,
            )
        );
    }


    validate_foreign_keys(
        connection
    )?;


    Ok(())
}


pub fn validate_startup(
    connection: &Connection,
) -> Result<(), String> {

    validate_startup_metadata(
        connection
    )?;


    validate_required_tables(
        connection
    )?;


    validate_foreign_key_enforcement(
        connection
    )?;


    validate_foreign_keys(
        connection
    )?;


    Ok(())
}


pub fn validate_initialization(
    connection: &Connection,
) -> Result<(), String> {

    validate_schema_metadata(
        connection
    )?;


    validate_textures(
        connection
    )?;


    validate_curated_palette(
        connection
    )?;


    validate_app_defaults(
        connection
    )?;


    validate_target_defaults(
        connection
    )?;


    let default_shader_id =
        validate_default_shader(
            connection
        )?;


    let default_screensaver_policy_id =
        validate_default_policy(
            connection,
            default_shader_id,
            "screensaver default",
            "screensaver",
        )?;


    let default_wallpaper_policy_id =
        validate_default_policy(
            connection,
            default_shader_id,
            "wallpaper default",
            "wallpaper",
        )?;


    validate_runtime_targets(
        connection,
        default_screensaver_policy_id,
        default_wallpaper_policy_id,
    )?;


    validate_integrity(
        connection
    )?;


    Ok(())
}


fn validate_startup_metadata(
    connection: &Connection,
) -> Result<(), String> {

    let metadata_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM schema_metadata",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to inspect schema_metadata during startup validation: {}",
                        error,
                    )
                }
            )?;


    if metadata_count
        != 1
    {
        return Err(
            format!(
                "Startup database validation failed: expected exactly one schema_metadata row, found {}",
                metadata_count,
            )
        );
    }


    let (
        metadata_id,
        schema_version,
    ): (
        i64,
        i64,
    ) =
        connection
            .query_row(
                "SELECT
                     metadata_id,
                     schema_version
                 FROM schema_metadata",
                [],
                |row| {
                    Ok(
                        (
                            row.get(
                                0
                            )?,
                            row.get(
                                1
                            )?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read schema metadata during startup validation: {}",
                        error,
                    )
                }
            )?;


    if metadata_id
        != 1
    {
        return Err(
            format!(
                "Startup database validation failed: expected metadata_id 1, found {}",
                metadata_id,
            )
        );
    }


    if schema_version
        < 1
    {
        return Err(
            format!(
                "Startup database validation failed: schema version {} is invalid",
                schema_version,
            )
        );
    }


    Ok(())
}


fn validate_required_tables(
    connection: &Connection,
) -> Result<(), String> {

    const REQUIRED_TABLES: [&str; 8] = [
        "schema_metadata",
        "shaders",
        "shader_policies",
        "runtime_targets",
        "app_defaults",
        "target_defaults",
        "textures",
        "curated_palette",
    ];


    for table_name in
        REQUIRED_TABLES
    {
        let table_count: i64 =
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM sqlite_master
                     WHERE type = 'table'
                       AND name = ?1",
                    [table_name],
                    |row| {
                        row.get(
                            0
                        )
                    },
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to verify required database table '{}': {}",
                            table_name,
                            error,
                        )
                    }
                )?;


        if table_count
            != 1
        {
            return Err(
                format!(
                    "Startup database validation failed: required table '{}' is missing",
                    table_name,
                )
            );
        }
    }


    Ok(())
}


fn validate_foreign_key_enforcement(
    connection: &Connection,
) -> Result<(), String> {

    let foreign_keys_enabled: i64 =
        connection
            .query_row(
                "PRAGMA foreign_keys",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to inspect SQLite foreign-key enforcement: {}",
                        error,
                    )
                }
            )?;


    if foreign_keys_enabled
        != 1
    {
        return Err(
            "Startup database validation failed: SQLite foreign-key enforcement is not enabled"
                .to_string()
        );
    }


    Ok(())
}


fn validate_foreign_keys(
    connection: &Connection,
) -> Result<(), String> {

    let foreign_key_violation_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM pragma_foreign_key_check",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to run SQLite foreign-key check: {}",
                        error,
                    )
                }
            )?;


    if foreign_key_violation_count
        != 0
    {
        return Err(
            format!(
                "SQLite foreign-key check failed: {} violation(s)",
                foreign_key_violation_count,
            )
        );
    }


    Ok(())
}


fn validate_schema_metadata(
    connection: &Connection,
) -> Result<(), String> {

    let metadata_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM schema_metadata
                 WHERE metadata_id = 1
                   AND schema_version = ?1",
                [EXPECTED_SCHEMA_VERSION],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to validate schema metadata: {}",
                        error,
                    )
                }
            )?;


    if metadata_count
        != 1
    {
        return Err(
            format!(
                "Schema metadata validation failed: expected exactly one Schema Version {} metadata row",
                EXPECTED_SCHEMA_VERSION,
            )
        );
    }


    Ok(())
}


fn validate_textures(
    connection: &Connection,
) -> Result<(), String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*) FROM textures",
                [],
                |row| row.get(0),
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count texture-catalog rows during initialization validation: {}",
                        error,
                    )
                }
            )?;


    let expected_count =
        crate::generate_textures::TextureFamily::ALL.len() as i64;


    if row_count != expected_count {
        return Err(
            format!(
                "Texture-catalog validation failed: expected {} rows, found {}",
                expected_count,
                row_count,
            )
        );
    }


    for (
        display_order,
        family,
    ) in crate::generate_textures::TextureFamily::ALL
        .iter()
        .enumerate()
    {
        let matching_count: i64 =
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM textures
                     WHERE texture_name = ?1
                       AND display_order = ?2",
                    params![
                        family.name(),
                        display_order as i64,
                    ],
                    |row| row.get(0),
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to validate texture family '{}': {}",
                            family.name(),
                            error,
                        )
                    }
                )?;


        if matching_count != 1 {
            return Err(
                format!(
                    "Texture-catalog validation failed: expected exactly one '{}' row at display order {}",
                    family.name(),
                    display_order,
                )
            );
        }
    }


    Ok(())
}


fn validate_curated_palette(
    connection: &Connection,
) -> Result<(), String> {

    let stored_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM curated_palette",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to validate curated-palette row count: {}",
                        error,
                    )
                }
            )?;


    let expected_count =
        crate::palettes::CURATED_PALETTE_COLORS
            .len() as i64;


    if stored_count
        != expected_count
    {
        return Err(
            format!(
                "Curated-palette validation failed: expected {} rows, found {}",
                expected_count,
                stored_count,
            )
        );
    }


    Ok(())
}


fn validate_app_defaults(
    connection: &Connection,
) -> Result<(), String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM app_defaults",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count application-default rows during initialization validation: {}",
                        error,
                    )
                }
            )?;


    if row_count
        != 1
    {
        return Err(
            format!(
                "Application-default validation failed: expected exactly 1 row, found {}",
                row_count,
            )
        );
    }


    let valid_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM app_defaults
                 WHERE defaults_id = 1
                   AND show_splash = 1
                   AND screensaver_subtitles = 1
                   AND subtitle_placement = 'bottom:center'
                   AND wallpaper_notifications = 1
                   AND rendered_fps = 30
                   AND anti_aliasing = 'fxaa'
                   AND dithering = 'subtle'
                   AND color_precision = 'auto'
                   AND render_scale = 1.0",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to validate initial application-default values: {}",
                        error,
                    )
                }
            )?;


    if valid_count
        != 1
    {
        return Err(
            "Application-default validation failed: initial values do not match Schema V1 factory defaults"
                .to_string()
        );
    }


    Ok(())
}


fn validate_target_defaults(
    connection: &Connection,
) -> Result<(), String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM target_defaults",
                [],
                |row| {
                    row.get(
                        0
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count target-default rows during initialization validation: {}",
                        error,
                    )
                }
            )?;


    if row_count
        != 2
    {
        return Err(
            format!(
                "Target-default validation failed: expected exactly 2 rows, found {}",
                row_count,
            )
        );
    }


    for (
        target,
        expected_idle_timeout_value,
        expected_idle_timeout_unit,
        expected_animation_speed,
    ) in [
        (
            "screensaver",
            Some(10_i64),
            Some("minutes"),
            1.0_f64,
        ),
        (
            "wallpaper",
            None,
            None,
            0.03_f64,
        ),
    ] {
        let (
            idle_timeout_value,
            idle_timeout_unit,
            animation_speed,
            texture_mode,
            texture_family,
            texture_primitives,
            palette_mode,
            palette_color,
        ): (
            Option<i64>,
            Option<String>,
            f64,
            String,
            Option<String>,
            i64,
            String,
            Option<String>,
        ) =
            connection
                .query_row(
                    "SELECT
                         idle_timeout_value,
                         idle_timeout_unit,
                         animation_speed,
                         texture_mode,
                         texture_family,
                         texture_primitives,
                         palette_mode,
                         palette_color
                     FROM target_defaults
                     WHERE target = ?1",
                    [target],
                    |row| {
                        Ok(
                            (
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                                row.get(5)?,
                                row.get(6)?,
                                row.get(7)?,
                            )
                        )
                    },
                )
                .map_err(
                    |error| {
                        format!(
                            "Target-default validation failed: unable to read '{}' row: {}",
                            target,
                            error,
                        )
                    }
                )?;


        if idle_timeout_value
                != expected_idle_timeout_value
            || idle_timeout_unit.as_deref()
                != expected_idle_timeout_unit
            || (
                animation_speed
                    - expected_animation_speed
            )
            .abs()
                > f64::EPSILON
            || texture_mode
                != "random"
            || texture_family
                .is_some()
            || texture_primitives
                != 64
            || palette_mode
                != "random"
            || palette_color
                .is_some()
        {
            return Err(
                format!(
                    "Target-default validation failed: '{}' initial values do not match Schema V1 factory defaults",
                    target,
                )
            );
        }
    }


    Ok(())
}


fn validate_default_shader(
    connection: &Connection,
) -> Result<i64, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     shader_id,
                     source_hash,
                     file_status,
                     validation_status,
                     preprocessed_source,
                     preprocessor_version,
                     channel_usage_mask,
                     shader_inputs_json
                 FROM shaders
                 WHERE filename = 'default.glsl'"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare default-shader validation query: {}",
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
                        "Unable to query default shader: {}",
                        error,
                    )
                }
            )?;


    let row =
        rows
            .next()
            .map_err(
                |error| {
                    format!(
                        "Unable to read default-shader validation result: {}",
                        error,
                    )
                }
            )?
            .ok_or_else(
                || {
                    "Default-shader validation failed: default.glsl is missing"
                        .to_string()
                }
            )?;


    let shader_id: i64 =
        row.get(
            0
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default shader_id: {}",
                    error,
                )
            }
        )?;


    let source_hash: String =
        row.get(
            1
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default source_hash: {}",
                    error,
                )
            }
        )?;


    let file_status: String =
        row.get(
            2
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default file_status: {}",
                    error,
                )
            }
        )?;


    let validation_status: String =
        row.get(
            3
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default validation_status: {}",
                    error,
                )
            }
        )?;


    let preprocessed_source: Vec<u8> =
        row.get(
            4
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default preprocessed_source: {}",
                    error,
                )
            }
        )?;


    let preprocessor_version: i64 =
        row.get(
            5
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default preprocessor_version: {}",
                    error,
                )
            }
        )?;


    let channel_usage_mask: i64 =
        row.get(
            6
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default channel_usage_mask: {}",
                    error,
                )
            }
        )?;


    let shader_inputs_json: String =
        row.get(
            7
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read default shader_inputs_json: {}",
                    error,
                )
            }
        )?;


    if rows
        .next()
        .map_err(
            |error| {
                format!(
                    "Unable to check for duplicate default shaders: {}",
                    error,
                )
            }
        )?
        .is_some()
    {
        return Err(
            "Default-shader validation failed: more than one default.glsl row exists"
                .to_string()
        );
    }


    if !is_lowercase_sha256(
        &source_hash
    )
    {
        return Err(
            format!(
                "Default-shader validation failed: source_hash '{}' is not a lowercase 64-character SHA-256 value",
                source_hash,
            )
        );
    }


    if file_status
        != "present"
    {
        return Err(
            format!(
                "Default-shader validation failed: expected file_status 'present', found '{}'",
                file_status,
            )
        );
    }


    if validation_status
        != "valid"
    {
        return Err(
            format!(
                "Default-shader validation failed: expected validation_status 'valid', found '{}'",
                validation_status,
            )
        );
    }


    if preprocessed_source
        .is_empty()
    {
        return Err(
            "Default-shader validation failed: runtime-source BLOB is empty"
                .to_string()
        );
    }


    if preprocessor_version
        != EXPECTED_RUNTIME_SOURCE_PREPARATION_VERSION
    {
        return Err(
            format!(
                "Default-shader validation failed: expected runtime-source preparation version {}, found {}",
                EXPECTED_RUNTIME_SOURCE_PREPARATION_VERSION,
                preprocessor_version,
            )
        );
    }


    if !(0..=31)
        .contains(
            &channel_usage_mask
        )
    {
        return Err(
            format!(
                "Default-shader validation failed: channel_usage_mask {} is outside the valid range 0..31",
                channel_usage_mask,
            )
        );
    }


    if shader_inputs_json
        != "[]"
    {
        return Err(
            format!(
                "Default-shader validation failed: native default.glsl must store an empty ShaderInput list, found '{}'",
                shader_inputs_json,
            )
        );
    }


    Ok(
        shader_id
    )
}


fn validate_default_policy(
    connection: &Connection,
    expected_shader_id: i64,
    policy_name_key: &str,
    policy_target: &str,
) -> Result<i64, String> {

    // Policy Name is display/search metadata, not policy identity.  Duplicate
    // names are valid in Schema V1, so default-policy validation must identify
    // the protected fallback row structurally: the oldest policy_id for the
    // managed default.glsl shader in the required target.
    let fallback =
        connection
            .query_row(
                "SELECT policy_id,
                        policy_name_key
                 FROM shader_policies
                 WHERE shader_id = ?1
                   AND policy_target = ?2
                 ORDER BY policy_id
                 LIMIT 1",
                rusqlite::params![
                    expected_shader_id,
                    policy_target,
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Default-policy validation failed: required '{}' fallback policy for shader_id {} is missing: {}",
                        policy_target,
                        expected_shader_id,
                        error,
                    )
                }
            )?;

    let (
        fallback_policy_id,
        stored_name_key,
    ) = fallback;

    // During fresh initialization the canonical fallback names should still be
    // the seeded names.  This is a content check only; it is deliberately not
    // a uniqueness check.  Other policies may freely use the same name/target.
    if stored_name_key
        != policy_name_key
    {
        return Err(
            format!(
                "Default-policy validation failed: protected {} fallback policy ID {} has Policy Name key '{}', expected seeded key '{}'",
                policy_target,
                fallback_policy_id,
                stored_name_key,
                policy_name_key,
            )
        );
    }

    Ok(
        fallback_policy_id
    )
}


fn validate_runtime_targets(
    connection: &Connection,
    expected_screensaver_policy_id: i64,
    expected_wallpaper_policy_id: i64,
) -> Result<(), String> {

    let row_count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*) FROM runtime_targets",
                [],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to count runtime-target rows during initialization validation: {}",
                        error,
                    )
                }
            )?;


    if row_count != 2 {
        return Err(
            format!(
                "Runtime-target validation failed: expected exactly 2 rows, found {}",
                row_count,
            )
        );
    }


    for (
        target,
        expected_policy_id,
    ) in [
        (
            "screensaver",
            expected_screensaver_policy_id,
        ),
        (
            "wallpaper",
            expected_wallpaper_policy_id,
        ),
    ] {
        let (
            display_mode,
            interval_seconds,
            single_policy_id,
            selected_policy_target,
        ): (
            String,
            Option<i64>,
            Option<i64>,
            Option<String>,
        ) =
            connection
                .query_row(
                    "SELECT
                         rt.display_mode,
                         rt.interval_seconds,
                         rt.single_policy_id,
                         p.policy_target
                     FROM runtime_targets AS rt
                     LEFT JOIN shader_policies AS p
                       ON p.policy_id = rt.single_policy_id
                     WHERE rt.target = ?1",
                    [target],
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
                            "Runtime-target validation failed: unable to read '{}' row: {}",
                            target,
                            error,
                        )
                    }
                )?;


        if display_mode != "single"
            || interval_seconds.is_some()
            || single_policy_id != Some(expected_policy_id)
            || selected_policy_target.as_deref() != Some(target)
        {
            return Err(
                format!(
                    "Runtime-target validation failed: '{}' must initially be Single using policy ID {} for the same target",
                    target,
                    expected_policy_id,
                )
            );
        }
    }


    Ok(())
}


fn is_lowercase_sha256(
    value: &str,
) -> bool {

    value.len() == 64
        && value
            .bytes()
            .all(
                |byte| {
                    byte.is_ascii_digit()
                        || (
                            b'a'..=b'f'
                        )
                        .contains(
                            &byte
                        )
                }
            )
}
