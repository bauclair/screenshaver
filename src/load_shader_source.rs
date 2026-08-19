use rusqlite::OptionalExtension;


const EXPECTED_RUNTIME_SOURCE_PREPARATION_VERSION: i64 = 1;


#[derive(Debug)]
pub enum ShaderSourceResult {

    Ready {
        source: String,
        shader_type: String,
        channel_usage:
            crate::preprocess_shader::ShaderChannelUsage,
        shader_inputs:
            Vec<crate::isf_types::ShaderInput>,
    },

    Rejected {
        reason: Option<String>,
        message: Option<String>,
    },

    Unavailable {
        error: String,
    },
}


pub fn load_managed_shader_source(
    shader_name: &str,
) -> Result<ShaderSourceResult, String> {

    let database_path =
        crate::locate_paths::database_path();


    if !database_path.exists() {

        return Err(
            format!(
                "Screenshaver database does not exist: {}",
                database_path.display(),
            )
        );
    }


    let connection =
        crate::open_database::open()?;


    let source_path =
        crate::locate_paths::shader_dir()
            .to_string_lossy()
            .to_string();


    let record =
        connection
            .query_row(
                "SELECT
                     shader_type,
                     file_status,
                     validation_status,
                     validation_reason,
                     validation_message,
                     preprocessed_source,
                     preprocessor_version,
                     channel_usage_mask,
                     shader_inputs_json
                 FROM shaders
                 WHERE source_path = ?1
                   AND filename = ?2",
                rusqlite::params![
                    source_path,
                    shader_name,
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, String>(
                                0
                            )?,
                            row.get::<_, String>(
                                1
                            )?,
                            row.get::<_, String>(
                                2
                            )?,
                            row.get::<_, Option<String>>(
                                3
                            )?,
                            row.get::<_, Option<String>>(
                                4
                            )?,
                            row.get::<_, Option<Vec<u8>>>(
                                5
                            )?,
                            row.get::<_, Option<i64>>(
                                6
                            )?,
                            row.get::<_, Option<i64>>(
                                7
                            )?,
                            row.get::<_, Option<String>>(
                                8
                            )?,
                        )
                    )
                },
            )
            .optional()
            .map_err(
                |error| {
                    format!(
                        "Unable to query runtime source for shader '{}': {}",
                        shader_name,
                        error,
                    )
                }
            )?;


    let Some(
        (
            shader_type,
            file_status,
            validation_status,
            validation_reason,
            validation_message,
            runtime_source,
            preprocessor_version,
            channel_usage_mask,
            shader_inputs_json,
        )
    ) =
        record
    else {

        return Ok(
            ShaderSourceResult::Unavailable {
                error:
                    format!(
                        "Shader '{}' is not registered in the Screenshaver database",
                        shader_name,
                    ),
            }
        );
    };


    if file_status
        != "present"
    {

        return Ok(
            ShaderSourceResult::Unavailable {
                error:
                    match file_status.as_str() {

                        "missing" => {
                            format!(
                                "Shader '{}' is registered but its physical source file is missing",
                                shader_name,
                            )
                        }

                        "unreadable" => {
                            format!(
                                "Shader '{}' is registered but its physical source file is unreadable",
                                shader_name,
                            )
                        }

                        _ => {
                            format!(
                                "Shader '{}' has unsupported file_status '{}'",
                                shader_name,
                                file_status,
                            )
                        }
                    },
            }
        );
    }


    if validation_status
        == "rejected"
    {

        return Ok(
            ShaderSourceResult::Rejected {
                reason:
                    validation_reason,

                message:
                    validation_message,
            }
        );
    }


    if validation_status
        != "valid"
    {

        return Ok(
            ShaderSourceResult::Unavailable {
                error:
                    format!(
                        "Shader '{}' has validation_status '{}'; expected 'valid'",
                        shader_name,
                        validation_status,
                    ),
            }
        );
    }


    let runtime_source =
        runtime_source
            .ok_or_else(
                || {
                    format!(
                        "Valid shader '{}' has no runtime-source BLOB",
                        shader_name,
                    )
                }
            )?;


    let preprocessor_version =
        preprocessor_version
            .ok_or_else(
                || {
                    format!(
                        "Valid shader '{}' has no runtime-source preparation version",
                        shader_name,
                    )
                }
            )?;


    if preprocessor_version
        != EXPECTED_RUNTIME_SOURCE_PREPARATION_VERSION
    {

        return Ok(
            ShaderSourceResult::Unavailable {
                error:
                    format!(
                        "Shader '{}' runtime source was prepared by version {}, but this executable requires version {}",
                        shader_name,
                        preprocessor_version,
                        EXPECTED_RUNTIME_SOURCE_PREPARATION_VERSION,
                    ),
            }
        );
    }


    let channel_usage_mask =
        channel_usage_mask
            .ok_or_else(
                || {
                    format!(
                        "Valid shader '{}' has no runtime channel-usage metadata",
                        shader_name,
                    )
                }
            )?;


    if !(0..=31)
        .contains(
            &channel_usage_mask
        )
    {

        return Ok(
            ShaderSourceResult::Unavailable {
                error:
                    format!(
                        "Shader '{}' has invalid runtime channel-usage mask {}",
                        shader_name,
                        channel_usage_mask,
                    ),
            }
        );
    }


    let shader_inputs_json =
        shader_inputs_json
            .ok_or_else(
                || {
                    format!(
                        "Valid shader '{}' has no runtime ShaderInput metadata",
                        shader_name,
                    )
                }
            )?;


    let channel_usage =
        decode_channel_usage(
            channel_usage_mask
        );


    let shader_inputs =
        deserialize_shader_inputs(
            shader_name,
            &shader_inputs_json,
        )?;


    let source =
        String::from_utf8(
            runtime_source
        )
        .map_err(
            |error| {
                format!(
                    "Runtime-source BLOB for shader '{}' is not valid UTF-8: {}",
                    shader_name,
                    error,
                )
            }
        )?;


    if source.is_empty() {

        return Ok(
            ShaderSourceResult::Unavailable {
                error:
                    format!(
                        "Runtime-source BLOB for shader '{}' is empty",
                        shader_name,
                    ),
            }
        );
    }


    Ok(
        ShaderSourceResult::Ready {
            source,
            shader_type,
            channel_usage,
            shader_inputs,
        }
    )
}


fn decode_channel_usage(
    mask: i64,
) -> crate::preprocess_shader::ShaderChannelUsage {

    crate::preprocess_shader::ShaderChannelUsage {
        channels: [
            mask & 1 != 0,
            mask & (1 << 1) != 0,
            mask & (1 << 2) != 0,
            mask & (1 << 3) != 0,
        ],

        requires_mipmaps:
            mask & (1 << 4) != 0,
    }
}


fn deserialize_shader_inputs(
    shader_name: &str,
    json: &str,
) -> Result<Vec<crate::isf_types::ShaderInput>, String> {

    let values: Vec<serde_json::Value> =
        serde_json::from_str(
            json
        )
        .map_err(
            |error| {
                format!(
                    "Runtime ShaderInput metadata for shader '{}' is invalid JSON: {}",
                    shader_name,
                    error,
                )
            }
        )?;


    let mut inputs =
        Vec::with_capacity(
            values.len()
        );


    for value in values {

        let object =
            value
                .as_object()
                .ok_or_else(
                    || {
                        format!(
                            "Runtime ShaderInput metadata for shader '{}' contains a non-object entry",
                            shader_name,
                        )
                    }
                )?;


        let name =
            object
                .get(
                    "name"
                )
                .and_then(
                    |value| {
                        value.as_str()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Runtime ShaderInput metadata for shader '{}' contains an entry without a valid name",
                            shader_name,
                        )
                    }
                )?
                .to_string();


        let input_type =
            object
                .get(
                    "type"
                )
                .and_then(
                    |value| {
                        value.as_str()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Runtime ShaderInput '{}' for shader '{}' has no valid type",
                            name,
                            shader_name,
                        )
                    }
                )?;


        let stored_value =
            object
                .get(
                    "value"
                )
                .ok_or_else(
                    || {
                        format!(
                            "Runtime ShaderInput '{}' for shader '{}' has no value",
                            name,
                            shader_name,
                        )
                    }
                )?;


        let input_value =
            match input_type {

                "float" => {

                    let number =
                        stored_value
                            .as_f64()
                            .ok_or_else(
                                || {
                                    format!(
                                        "Runtime ShaderInput '{}' for shader '{}' does not contain a valid float value",
                                        name,
                                        shader_name,
                                    )
                                }
                            )?;


                    crate::isf_types::ShaderInputValue::Float(
                        number as f32
                    )
                }


                "bool" => {

                    let boolean =
                        stored_value
                            .as_bool()
                            .ok_or_else(
                                || {
                                    format!(
                                        "Runtime ShaderInput '{}' for shader '{}' does not contain a valid bool value",
                                        name,
                                        shader_name,
                                    )
                                }
                            )?;


                    crate::isf_types::ShaderInputValue::Bool(
                        boolean
                    )
                }


                "integer" => {

                    let integer =
                        stored_value
                            .as_i64()
                            .ok_or_else(
                                || {
                                    format!(
                                        "Runtime ShaderInput '{}' for shader '{}' does not contain a valid integer value",
                                        name,
                                        shader_name,
                                    )
                                }
                            )?;


                    let integer =
                        i32::try_from(
                            integer
                        )
                        .map_err(
                            |_| {
                                format!(
                                    "Runtime ShaderInput '{}' for shader '{}' is outside the i32 range",
                                    name,
                                    shader_name,
                                )
                            }
                        )?;


                    crate::isf_types::ShaderInputValue::Integer(
                        integer
                    )
                }


                "point2d" => {

                    let values =
                        json_number_array::<2>(
                            shader_name,
                            &name,
                            stored_value,
                        )?;


                    crate::isf_types::ShaderInputValue::Point2D(
                        values
                    )
                }


                "color" => {

                    let values =
                        json_number_array::<4>(
                            shader_name,
                            &name,
                            stored_value,
                        )?;


                    crate::isf_types::ShaderInputValue::Color(
                        values
                    )
                }


                _ => {

                    return Err(
                        format!(
                            "Runtime ShaderInput '{}' for shader '{}' has unsupported type '{}'",
                            name,
                            shader_name,
                            input_type,
                        )
                    );
                }
            };


        inputs.push(
            crate::isf_types::ShaderInput {
                name,
                value:
                    input_value,
            }
        );
    }


    Ok(
        inputs
    )
}


fn json_number_array<const N: usize>(
    shader_name: &str,
    input_name: &str,
    value: &serde_json::Value,
) -> Result<[f32; N], String> {

    let array =
        value
            .as_array()
            .ok_or_else(
                || {
                    format!(
                        "Runtime ShaderInput '{}' for shader '{}' does not contain an array value",
                        input_name,
                        shader_name,
                    )
                }
            )?;


    if array.len()
        != N
    {

        return Err(
            format!(
                "Runtime ShaderInput '{}' for shader '{}' expected {} values, found {}",
                input_name,
                shader_name,
                N,
                array.len(),
            )
        );
    }


    let mut result =
        [0.0_f32; N];


    for (
        index,
        value,
    ) in
        array.iter().enumerate()
    {

        result[index] =
            value
                .as_f64()
                .ok_or_else(
                    || {
                        format!(
                            "Runtime ShaderInput '{}' for shader '{}' contains a non-numeric array value",
                            input_name,
                            shader_name,
                        )
                    }
                )?
                as f32;
    }


    Ok(
        result
    )
}

