use rusqlite::Connection;


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


pub fn validate_initialization(
    connection: &Connection,
) -> Result<(), String> {

    validate_schema_metadata(
        connection
    )?;


    validate_curated_palette(
        connection
    )?;


    let default_shader_id =
        validate_default_shader(
            connection
        )?;


    validate_default_policy(
        connection,
        default_shader_id,
        "default screensaver",
        "screensaver",
    )?;


    validate_default_policy(
        connection,
        default_shader_id,
        "default wallpaper",
        "wallpaper",
    )?;


    validate_integrity(
        connection
    )?;


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
                     preprocessor_version
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


    Ok(
        shader_id
    )
}


fn validate_default_policy(
    connection: &Connection,
    expected_shader_id: i64,
    policy_name_key: &str,
    policy_target: &str,
) -> Result<(), String> {

    let mut statement =
        connection
            .prepare(
                "SELECT shader_id,
                        policy_target
                 FROM shader_policies
                 WHERE policy_name_key = ?1"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare default-policy validation query for '{}': {}",
                        policy_name_key,
                        error,
                    )
                }
            )?;


    let mut rows =
        statement
            .query(
                [policy_name_key]
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query default policy '{}': {}",
                        policy_name_key,
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
                        "Unable to read default policy '{}': {}",
                        policy_name_key,
                        error,
                    )
                }
            )?
            .ok_or_else(
                || {
                    format!(
                        "Default-policy validation failed: '{}' is missing",
                        policy_name_key,
                    )
                }
            )?;


    let shader_id: i64 =
        row.get(
            0
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read shader_id for default policy '{}': {}",
                    policy_name_key,
                    error,
                )
            }
        )?;


    let stored_target: String =
        row.get(
            1
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read policy_target for default policy '{}': {}",
                    policy_name_key,
                    error,
                )
            }
        )?;


    if rows
        .next()
        .map_err(
            |error| {
                format!(
                    "Unable to check for duplicate default policy '{}': {}",
                    policy_name_key,
                    error,
                )
            }
        )?
        .is_some()
    {
        return Err(
            format!(
                "Default-policy validation failed: more than one '{}' policy exists",
                policy_name_key,
            )
        );
    }


    if shader_id
        != expected_shader_id
    {
        return Err(
            format!(
                "Default-policy validation failed: '{}' references shader_id {}, expected {}",
                policy_name_key,
                shader_id,
                expected_shader_id,
            )
        );
    }


    if stored_target
        != policy_target
    {
        return Err(
            format!(
                "Default-policy validation failed: '{}' target is '{}', expected '{}'",
                policy_name_key,
                stored_target,
                policy_target,
            )
        );
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
