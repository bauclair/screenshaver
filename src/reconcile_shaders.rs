use std::collections::{
    HashMap,
    HashSet,
};
use std::fs;
use std::path::{
    Path,
    PathBuf,
};

use rusqlite::{
    params,
    Connection,
};


const RUNTIME_SOURCE_PREPARATION_VERSION: i64 = 1;


#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
)]
pub struct ReconciliationOutcome {

    pub inserted:
        usize,

    pub updated:
        usize,

    pub unchanged:
        usize,

    pub marked_missing:
        usize,

    pub marked_unreadable:
        usize,

    pub rejected:
        usize,
}


#[derive(Debug)]
struct ExistingShader {

    shader_id:
        i64,

    source_hash:
        Option<String>,

    shader_type:
        String,

    file_status:
        String,

    validation_status:
        String,

    validation_reason:
        Option<String>,

    validation_message:
        Option<String>,

    runtime_source_present:
        bool,

    preprocessor_version:
        Option<i64>,

    channel_usage_present:
        bool,

    shader_inputs_present:
        bool,
}


#[derive(Debug)]
struct PhysicalShader {

    filename:
        String,

    path:
        PathBuf,

    source:
        Result<Vec<u8>, String>,
}


#[derive(Debug)]
struct PreparedShader {

    filename:
        String,

    shader_type:
        String,

    source_hash:
        String,

    validation_status:
        String,

    validation_reason:
        Option<String>,

    validation_message:
        Option<String>,

    runtime_source:
        Option<Vec<u8>>,

    preprocessor_version:
        Option<i64>,

    channel_usage_mask:
        Option<i64>,

    shader_inputs_json:
        Option<String>,
}


#[derive(Debug)]
enum Mutation {

    InsertPrepared(
        PreparedShader
    ),

    UpdatePrepared {
        shader_id: i64,
        prepared: PreparedShader,
    },

    InsertUnreadable {
        filename: String,
        message: String,
    },

    MarkUnreadable {
        shader_id: i64,
    },

    MarkPresent {
        shader_id: i64,
    },

    MarkMissing {
        shader_id: i64,
    },
}


pub fn reconcile(
    connection: &mut Connection,
) -> Result<ReconciliationOutcome, String> {

    let shader_directory =
        crate::locate_paths::shader_dir();


    let source_path =
        shader_directory
            .to_string_lossy()
            .to_string();


    let existing =
        load_existing_shaders(
            connection,
            &source_path,
        )?;


    let physical =
        scan_shader_directory(
            &shader_directory
        )?;


    let mut seen_filenames =
        HashSet::<String>::new();


    let mut mutations =
        Vec::<Mutation>::new();


    let mut outcome =
        ReconciliationOutcome::default();


    for physical_shader in
        physical
    {
        seen_filenames.insert(
            physical_shader
                .filename
                .clone()
        );


        let existing_shader =
            existing.get(
                &physical_shader.filename
            );


        let source_bytes =
            match physical_shader.source {

                Ok(source) => {
                    source
                }

                Err(error) => {

                    outcome.marked_unreadable +=
                        1;


                    if let Some(existing_shader) =
                        existing_shader
                    {
                        if existing_shader.file_status
                            != "unreadable"
                        {
                            mutations.push(
                                Mutation::MarkUnreadable {
                                    shader_id:
                                        existing_shader.shader_id,
                                }
                            );
                        } else {
                            outcome.unchanged +=
                                1;
                        }
                    } else {
                        mutations.push(
                            Mutation::InsertUnreadable {
                                filename:
                                    physical_shader.filename,
                                message:
                                    error,
                            }
                        );

                        outcome.inserted +=
                            1;
                    }


                    continue;
                }
            };


        let source_hash =
            crate::hash_shader::hash_source(
                &source_bytes
            );


        if let Some(existing_shader) =
            existing_shader
        {
            if existing_shader.source_hash
                .as_deref()
                == Some(
                    source_hash.as_str()
                )
                && existing_record_can_be_reused(
                    existing_shader
                )
            {
                if existing_shader.file_status
                    != "present"
                {
                    mutations.push(
                        Mutation::MarkPresent {
                            shader_id:
                                existing_shader.shader_id,
                        }
                    );

                    outcome.updated +=
                        1;
                } else {
                    outcome.unchanged +=
                        1;
                }


                if existing_shader.validation_status
                    == "rejected"
                {
                    outcome.rejected +=
                        1;
                }


                continue;
            }
        }


        let prepared =
            prepare_shader(
                &physical_shader.filename,
                &source_bytes,
                source_hash,
            );


        if prepared.validation_status
            == "rejected"
        {
            outcome.rejected +=
                1;
        }


        if let Some(existing_shader) =
            existing_shader
        {
            if existing_shader.source_hash
                .as_deref()
                == Some(
                    prepared.source_hash.as_str()
                )
                && rejected_record_matches(
                    existing_shader,
                    &prepared,
                )
            {
                if existing_shader.file_status
                    != "present"
                {
                    mutations.push(
                        Mutation::MarkPresent {
                            shader_id:
                                existing_shader.shader_id,
                        }
                    );

                    outcome.updated +=
                        1;
                } else {
                    outcome.unchanged +=
                        1;
                }


                continue;
            }
        }


        match existing_shader {

            Some(existing_shader) => {

                mutations.push(
                    Mutation::UpdatePrepared {
                        shader_id:
                            existing_shader.shader_id,
                        prepared,
                    }
                );

                outcome.updated +=
                    1;
            }

            None => {

                mutations.push(
                    Mutation::InsertPrepared(
                        prepared
                    )
                );

                outcome.inserted +=
                    1;
            }
        }
    }


    for (
        filename,
        existing_shader,
    ) in
        &existing
    {
        if seen_filenames.contains(
            filename
        ) {
            continue;
        }


        if existing_shader.file_status
            == "missing"
        {
            outcome.unchanged +=
                1;

            continue;
        }


        mutations.push(
            Mutation::MarkMissing {
                shader_id:
                    existing_shader.shader_id,
            }
        );

        outcome.marked_missing +=
            1;
    }


    apply_mutations(
        connection,
        &source_path,
        &mutations,
    )?;


    Ok(
        outcome
    )
}


fn load_existing_shaders(
    connection: &Connection,
    source_path: &str,
) -> Result<
    HashMap<
        String,
        ExistingShader,
    >,
    String,
> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     shader_id,
                     filename,
                     source_hash,
                     shader_type,
                     file_status,
                     validation_status,
                     validation_reason,
                     validation_message,
                     preprocessed_source IS NOT NULL,
                     preprocessor_version,
                     channel_usage_mask IS NOT NULL,
                     shader_inputs_json IS NOT NULL
                 FROM shaders
                 WHERE source_path = ?1"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare shader reconciliation lookup: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [source_path],
                |row| {
                    Ok(
                        (
                            row.get::<_, i64>(
                                0
                            )?,
                            row.get::<_, String>(
                                1
                            )?,
                            row.get::<_, Option<String>>(
                                2
                            )?,
                            row.get::<_, String>(
                                3
                            )?,
                            row.get::<_, String>(
                                4
                            )?,
                            row.get::<_, String>(
                                5
                            )?,
                            row.get::<_, Option<String>>(
                                6
                            )?,
                            row.get::<_, Option<String>>(
                                7
                            )?,
                            row.get::<_, i64>(
                                8
                            )?
                                != 0,
                            row.get::<_, Option<i64>>(
                                9
                            )?,
                            row.get::<_, i64>(
                                10
                            )?
                                != 0,
                            row.get::<_, i64>(
                                11
                            )?
                                != 0,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query existing shader records: {}",
                        error,
                    )
                }
            )?;


    let mut existing =
        HashMap::new();


    for row in
        rows
    {
        let (
            shader_id,
            filename,
            source_hash,
            shader_type,
            file_status,
            validation_status,
            validation_reason,
            validation_message,
            runtime_source_present,
            preprocessor_version,
            channel_usage_present,
            shader_inputs_present,
        ) =
            row.map_err(
                |error| {
                    format!(
                        "Unable to read an existing shader record: {}",
                        error,
                    )
                }
            )?;


        existing.insert(
            filename,
            ExistingShader {
                shader_id,
                source_hash,
                shader_type,
                file_status,
                validation_status,
                validation_reason,
                validation_message,
                runtime_source_present,
                preprocessor_version,
                channel_usage_present,
                shader_inputs_present,
            },
        );
    }


    Ok(
        existing
    )
}


fn scan_shader_directory(
    shader_directory: &Path,
) -> Result<
    Vec<PhysicalShader>,
    String,
> {

    let entries =
        fs::read_dir(
            shader_directory
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read shader directory '{}': {}",
                    shader_directory.display(),
                    error,
                )
            }
        )?;


    let mut paths =
        Vec::<PathBuf>::new();


    for entry in
        entries
    {
        let entry =
            entry.map_err(
                |error| {
                    format!(
                        "Unable to read an entry in shader directory '{}': {}",
                        shader_directory.display(),
                        error,
                    )
                }
            )?;


        let path =
            entry.path();


        if !path.is_file() {
            continue;
        }


        let Some(filename) =
            path.file_name()
                .and_then(
                    |value| {
                        value.to_str()
                    }
                )
        else {
            continue;
        };


        if filename.contains(
            "._gen"
        ) {
            continue;
        }


        let extension =
            path.extension()
                .and_then(
                    |value| {
                        value.to_str()
                    }
                )
                .unwrap_or_default();


        if !extension.eq_ignore_ascii_case(
            "glsl"
        )
            && !extension.eq_ignore_ascii_case(
                "fs"
            )
            && !extension.eq_ignore_ascii_case(
                "shaver"
            )
        {
            continue;
        }


        paths.push(
            path
        );
    }


    paths.sort_by(
        |left, right| {
            left.file_name()
                .cmp(
                    &right.file_name()
                )
        }
    );


    let mut physical =
        Vec::with_capacity(
            paths.len()
        );


    for path in
        paths
    {
        let filename =
            path.file_name()
                .and_then(
                    |value| {
                        value.to_str()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Shader path '{}' has no valid UTF-8 filename",
                            path.display(),
                        )
                    }
                )?
                .to_string();


        let source =
            fs::read(
                &path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read shader '{}': {}",
                        path.display(),
                        error,
                    )
                }
            );


        physical.push(
            PhysicalShader {
                filename,
                path,
                source,
            }
        );
    }


    Ok(
        physical
    )
}


fn existing_record_can_be_reused(
    existing: &ExistingShader,
) -> bool {

    match existing.validation_status
        .as_str()
    {
        "valid" => {
            existing.runtime_source_present
                && existing.preprocessor_version
                    == Some(
                        RUNTIME_SOURCE_PREPARATION_VERSION
                    )
                && existing.channel_usage_present
                && existing.shader_inputs_present
        }

        "rejected" => {
            true
        }

        _ => {
            false
        }
    }
}


fn rejected_record_matches(
    existing: &ExistingShader,
    prepared: &PreparedShader,
) -> bool {

    existing.validation_status
        == "rejected"
        && prepared.validation_status
            == "rejected"
        && existing.shader_type
            == prepared.shader_type
        && existing.validation_reason
            == prepared.validation_reason
        && existing.validation_message
            == prepared.validation_message
        && !existing.runtime_source_present
        && existing.preprocessor_version
            .is_none()
        && !existing.channel_usage_present
        && !existing.shader_inputs_present
}


fn prepare_shader(
    filename: &str,
    source_bytes: &[u8],
    source_hash: String,
) -> PreparedShader {

    let source =
        match String::from_utf8(
            source_bytes.to_vec()
        ) {

            Ok(source) => {
                source
            }

            Err(error) => {

                return rejected_prepared_shader(
                    filename,
                    "unknown",
                    source_hash,
                    "invalid_utf8",
                    format!(
                        "Shader source is not valid UTF-8: {}",
                        error,
                    ),
                );
            }
        };


    match crate::classify_shader::classify_shader(
        &source
    ) {

        crate::classify_shader::ShaderKind::NativeGLSL => {

            prepare_native_shader(
                filename,
                source_bytes,
                &source,
                source_hash,
            )
        }


        crate::classify_shader::ShaderKind::ShaderToy => {

            prepare_shadertoy_shader(
                filename,
                &source,
                source_hash,
            )
        }


        crate::classify_shader::ShaderKind::Isf => {

            prepare_isf_shader(
                filename,
                &source,
                source_hash,
            )
        }
    }
}


fn prepare_native_shader(
    filename: &str,
    source_bytes: &[u8],
    source: &str,
    source_hash: String,
) -> PreparedShader {

    let (
        _warnings,
        rejection_reasons,
    ) =
        crate::preprocess_shader::analyze_native_shader(
            source
        );


    if !rejection_reasons.is_empty() {

        return rejected_prepared_shader(
            filename,
            "native",
            source_hash,
            "static_analysis_failed",
            rejection_reasons.join(
                "; "
            ),
        );
    }


    let channel_usage =
        crate::preprocess_shader::analyze_native_channel_usage(
            source
        );


    valid_prepared_shader(
        filename,
        "native",
        source_hash,
        source_bytes.to_vec(),
        channel_usage_mask(
            channel_usage
        ),
        "[]".to_string(),
    )
}


fn prepare_shadertoy_shader(
    filename: &str,
    source: &str,
    source_hash: String,
) -> PreparedShader {

    let report =
        crate::preprocess_shader::preprocess_shader_with_report(
            source
        );


    if !report.rejection_reasons
        .is_empty()
    {
        return rejected_prepared_shader(
            filename,
            "shadertoy",
            source_hash,
            "preprocessing_rejected",
            report.rejection_reasons
                .join(
                    "; "
                ),
        );
    }


    let channel_usage_mask =
        channel_usage_mask(
            report.channel_usage
        );


    valid_prepared_shader(
        filename,
        "shadertoy",
        source_hash,
        report.source.into_bytes(),
        channel_usage_mask,
        "[]".to_string(),
    )
}


fn prepare_isf_shader(
    filename: &str,
    source: &str,
    source_hash: String,
) -> PreparedShader {

    let document =
        match crate::parse_isf::parse(
            source
        ) {

            Ok(document) => {
                document
            }

            Err(error) => {

                return rejected_prepared_shader(
                    filename,
                    "isf",
                    source_hash,
                    "isf_parse_failed",
                    format!(
                        "ISF metadata parsing failed: {}",
                        error,
                    ),
                );
            }
        };


    let report =
        crate::preprocess_isf::preprocess(
            &document
        );


    if !report.rejection_reasons
        .is_empty()
    {
        return rejected_prepared_shader(
            filename,
            "isf",
            source_hash,
            "preprocessing_rejected",
            report.rejection_reasons
                .join(
                    "; "
                ),
        );
    }


    let shader_inputs_json =
        serialize_shader_inputs(
            &report.inputs
        );


    valid_prepared_shader(
        filename,
        "isf",
        source_hash,
        report.source.into_bytes(),
        0,
        shader_inputs_json,
    )
}



fn channel_usage_mask(
    usage: crate::preprocess_shader::ShaderChannelUsage,
) -> i64 {

    (if usage.channels[0] { 1 } else { 0 })
        | (if usage.channels[1] { 1 << 1 } else { 0 })
        | (if usage.channels[2] { 1 << 2 } else { 0 })
        | (if usage.channels[3] { 1 << 3 } else { 0 })
        | (if usage.requires_mipmaps { 1 << 4 } else { 0 })
}


fn serialize_shader_inputs(
    inputs: &[crate::isf_types::ShaderInput],
) -> String {

    let values =
        inputs
            .iter()
            .map(
                |input| {

                    let (
                        input_type,
                        value,
                    ) =
                        match &input.value {

                            crate::isf_types::ShaderInputValue::Float(
                                value
                            ) => {
                                (
                                    "float",
                                    serde_json::json!(
                                        value
                                    ),
                                )
                            }

                            crate::isf_types::ShaderInputValue::Bool(
                                value
                            ) => {
                                (
                                    "bool",
                                    serde_json::json!(
                                        value
                                    ),
                                )
                            }

                            crate::isf_types::ShaderInputValue::Integer(
                                value
                            ) => {
                                (
                                    "integer",
                                    serde_json::json!(
                                        value
                                    ),
                                )
                            }

                            crate::isf_types::ShaderInputValue::Point2D(
                                value
                            ) => {
                                (
                                    "point2d",
                                    serde_json::json!(
                                        value
                                    ),
                                )
                            }

                            crate::isf_types::ShaderInputValue::Color(
                                value
                            ) => {
                                (
                                    "color",
                                    serde_json::json!(
                                        value
                                    ),
                                )
                            }
                        };


                    serde_json::json!(
                        {
                            "name": input.name,
                            "type": input_type,
                            "value": value,
                        }
                    )
                }
            )
            .collect::<Vec<_>>();


    serde_json::Value::Array(
        values
    )
        .to_string()
}


fn valid_prepared_shader(
    filename: &str,
    shader_type: &str,
    source_hash: String,
    runtime_source: Vec<u8>,
    channel_usage_mask: i64,
    shader_inputs_json: String,
) -> PreparedShader {

    PreparedShader {
        filename:
            filename.to_string(),

        shader_type:
            shader_type.to_string(),

        source_hash,

        validation_status:
            "valid".to_string(),

        validation_reason:
            None,

        validation_message:
            None,

        runtime_source:
            Some(
                runtime_source
            ),

        preprocessor_version:
            Some(
                RUNTIME_SOURCE_PREPARATION_VERSION
            ),

        channel_usage_mask:
            Some(
                channel_usage_mask
            ),

        shader_inputs_json:
            Some(
                shader_inputs_json
            ),
    }
}


fn rejected_prepared_shader(
    filename: &str,
    shader_type: &str,
    source_hash: String,
    validation_reason: &str,
    validation_message: String,
) -> PreparedShader {

    PreparedShader {
        filename:
            filename.to_string(),

        shader_type:
            shader_type.to_string(),

        source_hash,

        validation_status:
            "rejected".to_string(),

        validation_reason:
            Some(
                validation_reason.to_string()
            ),

        validation_message:
            Some(
                validation_message
            ),

        runtime_source:
            None,

        preprocessor_version:
            None,

        channel_usage_mask:
            None,

        shader_inputs_json:
            None,
    }
}


fn apply_mutations(
    connection: &mut Connection,
    source_path: &str,
    mutations: &[Mutation],
) -> Result<(), String> {

    if mutations.is_empty() {
        return Ok(());
    }


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin shader reconciliation transaction: {}",
                        error,
                    )
                }
            )?;


    for mutation in
        mutations
    {
        match mutation {

            Mutation::InsertPrepared(
                prepared
            ) => {

                transaction
                    .execute(
                        "INSERT INTO shaders (
                             filename,
                             source_path,
                             shader_type,
                             source_hash,
                             file_status,
                             validation_status,
                             validation_reason,
                             validation_message,
                             preprocessed_source,
                             preprocessor_version,
                             channel_usage_mask,
                             shader_inputs_json
                         )
                         VALUES (
                             ?1,
                             ?2,
                             ?3,
                             ?4,
                             'present',
                             ?5,
                             ?6,
                             ?7,
                             ?8,
                             ?9,
                             ?10,
                             ?11
                         )",
                        params![
                            prepared.filename,
                            source_path,
                            prepared.shader_type,
                            prepared.source_hash,
                            prepared.validation_status,
                            prepared.validation_reason,
                            prepared.validation_message,
                            prepared.runtime_source,
                            prepared.preprocessor_version,
                            prepared.channel_usage_mask,
                            prepared.shader_inputs_json,
                        ],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to insert reconciled shader '{}': {}",
                                prepared.filename,
                                error,
                            )
                        }
                    )?;
            }


            Mutation::UpdatePrepared {
                shader_id,
                prepared,
            } => {

                transaction
                    .execute(
                        "UPDATE shaders
                         SET shader_type = ?1,
                             source_hash = ?2,
                             file_status = 'present',
                             validation_status = ?3,
                             validation_reason = ?4,
                             validation_message = ?5,
                             preprocessed_source = ?6,
                             preprocessor_version = ?7,
                             channel_usage_mask = ?8,
                             shader_inputs_json = ?9
                         WHERE shader_id = ?10",
                        params![
                            prepared.shader_type,
                            prepared.source_hash,
                            prepared.validation_status,
                            prepared.validation_reason,
                            prepared.validation_message,
                            prepared.runtime_source,
                            prepared.preprocessor_version,
                            prepared.channel_usage_mask,
                            prepared.shader_inputs_json,
                            shader_id,
                        ],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to update reconciled shader '{}' (shader_id {}): {}",
                                prepared.filename,
                                shader_id,
                                error,
                            )
                        }
                    )?;
            }


            Mutation::InsertUnreadable {
                filename,
                message,
            } => {

                transaction
                    .execute(
                        "INSERT INTO shaders (
                             filename,
                             source_path,
                             shader_type,
                             source_hash,
                             file_status,
                             validation_status,
                             validation_reason,
                             validation_message,
                             preprocessed_source,
                             preprocessor_version,
                             channel_usage_mask,
                             shader_inputs_json
                         )
                         VALUES (
                             ?1,
                             ?2,
                             'unknown',
                             NULL,
                             'unreadable',
                             'unknown',
                             NULL,
                             ?3,
                             NULL,
                             NULL,
                             NULL,
                             NULL
                         )",
                        params![
                            filename,
                            source_path,
                            message,
                        ],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to register unreadable shader '{}': {}",
                                filename,
                                error,
                            )
                        }
                    )?;
            }


            Mutation::MarkUnreadable {
                shader_id,
            } => {

                transaction
                    .execute(
                        "UPDATE shaders
                         SET file_status = 'unreadable'
                         WHERE shader_id = ?1",
                        [shader_id],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to mark shader_id {} unreadable: {}",
                                shader_id,
                                error,
                            )
                        }
                    )?;
            }


            Mutation::MarkPresent {
                shader_id,
            } => {

                transaction
                    .execute(
                        "UPDATE shaders
                         SET file_status = 'present'
                         WHERE shader_id = ?1",
                        [shader_id],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to mark shader_id {} present: {}",
                                shader_id,
                                error,
                            )
                        }
                    )?;
            }


            Mutation::MarkMissing {
                shader_id,
            } => {

                transaction
                    .execute(
                        "UPDATE shaders
                         SET file_status = 'missing'
                         WHERE shader_id = ?1",
                        [shader_id],
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to mark shader_id {} missing: {}",
                                shader_id,
                                error,
                            )
                        }
                    )?;
            }
        }
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit shader reconciliation transaction: {}",
                    error,
                )
            }
        )?;


    Ok(())
}
