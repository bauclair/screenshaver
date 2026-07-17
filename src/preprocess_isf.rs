//! Convert a supported single-pass ISF generator into GLSL 330 Core.

use regex::Regex;

#[derive(Debug, Default)]
pub struct IsfPreprocessResult {
    pub source: String,
    pub inputs: Vec<crate::isf_types::ShaderInput>,
    pub applied: Vec<String>,
    pub warnings: Vec<String>,
    pub rejection_reasons: Vec<String>,
}

pub fn preprocess(
    document: &crate::isf_types::IsfDocument,
) -> IsfPreprocessResult {
    let mut result =
        IsfPreprocessResult::default();

    if document.metadata.pass_count() != 1 {
        result.rejection_reasons.push(
            "ISF multipass shaders are not supported in this implementation"
                .to_string(),
        );
    }

    if document.metadata.has_imported_resources() {
        result.rejection_reasons.push(
            "ISF imported resources are not supported in this implementation"
                .to_string(),
        );
    }

    for metadata in
        &document.metadata.inputs
    {
        match input_from_metadata(
            metadata
        ) {
            Ok(input) => {
                result.inputs.push(
                    input
                )
            }

            Err(error) => {
                result.rejection_reasons.push(
                    error
                )
            }
        }
    }

    if !result.rejection_reasons.is_empty() {
        return result;
    }

    let mut body =
        normalize_line_endings(
            &document.shader_source
        );

    body =
        remove_version_precision_and_es_extensions(
            &body,
            &mut result.applied,
        );

    body =
        remove_redundant_standard_aliases(
            &body,
            &mut result.applied,
        );

    body =
        body.replace(
            "gl_FragColor",
            "fragColor"
        );

    result.applied.push(
        "ISF-COMP-0001:replace-gl_FragColor"
            .to_string()
    );

    body =
        initialize_main_output(
            &body,
            &mut result.applied,
        );

    let uniform_lines =
        result
            .inputs
            .iter()
            .map(
                |input| {
                    format!(
                        "uniform {} {};",
                        input.value.glsl_type(),
                        input.name,
                    )
                }
            )
            .collect::<Vec<_>>()
            .join(
                "\n"
            );

    let header =
        format!(
            "#version 330 core\n\
             out vec4 fragColor;\n\
             uniform float iTime;\n\
             uniform float iTimeDelta;\n\
             uniform vec3 iResolution;\n\
             uniform int iFrame;\n\
             #define TIME iTime\n\
             #define TIMEDELTA iTimeDelta\n\
             #define FRAMEINDEX iFrame\n\
             #define RENDERSIZE iResolution.xy\n\
             #define isf_FragNormCoord (gl_FragCoord.xy / iResolution.xy)\n\
             {}\n",
            uniform_lines,
        );

    result.applied.push(
        "ISF-COMP-0002:inject-standard-uniform-aliases"
            .to_string()
    );

    result.applied.push(
        "ISF-COMP-0003:inject-metadata-input-uniforms"
            .to_string()
    );

    result.source =
        format!(
            "{}\n{}",
            header,
            body,
        );

    result
}

fn input_from_metadata(
    metadata: &crate::isf_types::IsfInputMetadata,
) -> Result<
    crate::isf_types::ShaderInput,
    String,
> {
    if !is_valid_identifier(
        &metadata.name
    ) {
        return Err(
            format!(
                "ISF input has an invalid GLSL identifier: {}",
                metadata.name,
            )
        );
    }

    let normalized =
        metadata
            .input_type
            .trim()
            .to_ascii_lowercase();

    let value =
        match normalized.as_str() {
            "float" => {
                crate::isf_types::ShaderInputValue::Float(
                    default_float(
                        metadata
                    )
                )
            }

            "bool" => {
                crate::isf_types::ShaderInputValue::Bool(
                    default_bool(
                        metadata
                    )
                )
            }

            "long" => {
                crate::isf_types::ShaderInputValue::Integer(
                    default_integer(
                        metadata
                    )
                )
            }

            "point2d" => {
                crate::isf_types::ShaderInputValue::Point2D(
                    default_vector::<2>(
                        metadata
                    )
                )
            }

            "color" => {
                crate::isf_types::ShaderInputValue::Color(
                    default_vector::<4>(
                        metadata
                    )
                )
            }

            "image" => {
                return Err(
                    format!(
                        "ISF image input '{}' is not supported yet",
                        metadata.name,
                    )
                );
            }

            "audio"
            | "audiofft" => {
                return Err(
                    format!(
                        "ISF audio input '{}' is not supported yet",
                        metadata.name,
                    )
                );
            }

            "event" => {
                return Err(
                    format!(
                        "ISF event input '{}' is not supported yet",
                        metadata.name,
                    )
                );
            }

            other => {
                return Err(
                    format!(
                        "Unsupported ISF input type '{}' for '{}'",
                        other,
                        metadata.name,
                    )
                );
            }
        };

    Ok(
        crate::isf_types::ShaderInput {
            name:
                metadata.name.clone(),

            value,
        }
    )
}

fn default_float(
    metadata: &crate::isf_types::IsfInputMetadata,
) -> f32 {
    number(
        &metadata.default_value
    )
    .or_else(
        || {
            fallback_number(
                metadata
            )
        }
    )
    .unwrap_or(
        0.0
    )
        as f32
}

fn default_integer(
    metadata: &crate::isf_types::IsfInputMetadata,
) -> i32 {
    number(
        &metadata.default_value
    )
    .or_else(
        || {
            fallback_number(
                metadata
            )
        }
    )
    .unwrap_or(
        0.0
    )
    .round()
        as i32
}

fn default_bool(
    metadata: &crate::isf_types::IsfInputMetadata,
) -> bool {
    match &metadata.default_value {
        serde_json::Value::Bool(
            value
        ) => {
            *value
        }

        value => {
            number(
                value
            )
            .unwrap_or(
                0.0
            )
                != 0.0
        }
    }
}

fn default_vector<const N: usize>(
    metadata: &crate::isf_types::IsfInputMetadata,
) -> [f32; N] {
    let source =
        metadata.default_value.as_array();

    let minimum =
        metadata.minimum.as_array();

    let maximum =
        metadata.maximum.as_array();

    std::array::from_fn(
        |index| {
            source
                .and_then(
                    |values| {
                        values.get(
                            index
                        )
                    }
                )
                .and_then(
                    number
                )
                .or_else(
                    || {
                        let low =
                            minimum
                                .and_then(
                                    |values| {
                                        values.get(
                                            index
                                        )
                                    }
                                )
                                .and_then(
                                    number
                                );

                        let high =
                            maximum
                                .and_then(
                                    |values| {
                                        values.get(
                                            index
                                        )
                                    }
                                )
                                .and_then(
                                    number
                                );

                        match (
                            low,
                            high,
                        ) {
                            (
                                Some(low),
                                Some(high),
                            )
                                if low <= 0.0
                                    && high >= 0.0 =>
                            {
                                Some(
                                    0.0
                                )
                            }

                            (
                                Some(low),
                                Some(high),
                            ) => {
                                Some(
                                    (
                                        low + high
                                    )
                                        * 0.5
                                )
                            }

                            _ => {
                                None
                            }
                        }
                    }
                )
                .unwrap_or(
                    0.0
                )
                    as f32
        }
    )
}

fn fallback_number(
    metadata: &crate::isf_types::IsfInputMetadata,
) -> Option<f64> {
    let low =
        number(
            &metadata.minimum
        );

    let high =
        number(
            &metadata.maximum
        );

    match (
        low,
        high,
    ) {
        (
            Some(low),
            Some(high),
        )
            if low <= 0.0
                && high >= 0.0 =>
        {
            Some(
                0.0
            )
        }

        (
            Some(low),
            Some(high),
        ) => {
            Some(
                (
                    low + high
                )
                    * 0.5
            )
        }

        (
            Some(low),
            None,
        ) => {
            Some(
                low
            )
        }

        (
            None,
            Some(high),
        ) => {
            Some(
                high
            )
        }

        _ => {
            None
        }
    }
}

fn number(
    value: &serde_json::Value,
) -> Option<f64> {
    value.as_f64()
}

fn is_valid_identifier(
    value: &str,
) -> bool {
    let mut chars =
        value.chars();

    matches!(
        chars.next(),
        Some(first)
            if first == '_'
                || first.is_ascii_alphabetic()
    )
        && chars.all(
            |character| {
                character == '_'
                    || character.is_ascii_alphanumeric()
            }
        )
}

fn normalize_line_endings(
    source: &str,
) -> String {
    source
        .replace(
            "\r\n",
            "\n"
        )
        .replace(
            '\r',
            "\n"
        )
}

fn remove_version_precision_and_es_extensions(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    let gl_es_block =
        Regex::new(
            r"(?ms)^\s*#ifdef\s+GL_ES\s*\n\s*precision\s+\w+\s+float\s*;\s*\n\s*#endif\s*\n?"
        )
        .expect(
            "ISF GL_ES precision block regex"
        );

    let without_block =
        gl_es_block.replace_all(
            source,
            ""
        );

    if without_block.as_ref()
        != source
    {
        applied.push(
            "ISF-COMP-0005:remove-GL_ES-precision-block"
                .to_string()
        );
    }

    let mut removed_version =
        false;

    let mut removed_precision =
        false;

    let mut removed_extension =
        false;

    let output =
        without_block
            .lines()
            .filter(
                |line| {
                    let trimmed =
                        line.trim_start();

                    if trimmed.starts_with(
                        "#version"
                    ) {
                        removed_version =
                            true;

                        return false;
                    }

                    if trimmed.starts_with(
                        "precision "
                    ) {
                        removed_precision =
                            true;

                        return false;
                    }

                    if trimmed.starts_with(
                        "#extension"
                    )
                        && trimmed.contains(
                            "GL_OES_standard_derivatives"
                        )
                    {
                        removed_extension =
                            true;

                        return false;
                    }

                    true
                }
            )
            .collect::<Vec<_>>()
            .join(
                "\n"
            );

    if removed_version {
        applied.push(
            "ISF-COMP-0006:remove-version-directive"
                .to_string()
        );
    }

    if removed_precision {
        applied.push(
            "ISF-COMP-0007:remove-precision-declaration"
                .to_string()
        );
    }

    if removed_extension {
        applied.push(
            "ISF-COMP-0008:remove-GL_OES-standard-derivatives"
                .to_string()
        );
    }

    output
}

fn remove_redundant_standard_aliases(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    /*
        Deliberately matches only the standard ISF compatibility
        declaration:

            vec3 iResolution = vec3(RENDERSIZE, 1.0);

        Custom iResolution expressions are left unchanged.
    */

    let resolution_alias =
        Regex::new(
            r"(?m)^[ \t]*(?:const[ \t]+)?vec3[ \t]+iResolution[ \t]*=[ \t]*vec3[ \t]*\([ \t]*RENDERSIZE[ \t]*,[ \t]*1(?:\.0*)?[ \t]*\)[ \t]*;[ \t]*(?://[^\n]*)?\n?"
        )
        .expect(
            "ISF redundant iResolution alias regex"
        );

    let without_resolution =
        resolution_alias.replace_all(
            source,
            ""
        );

    if without_resolution.as_ref()
        != source
    {
        applied.push(
            "ISF-COMP-0009:remove-redundant-iResolution-alias"
                .to_string()
        );
    }

    /*
        Deliberately matches only:

            float iTime = TIME;
    */

    let time_alias =
        Regex::new(
            r"(?m)^[ \t]*(?:const[ \t]+)?float[ \t]+iTime[ \t]*=[ \t]*TIME[ \t]*;[ \t]*(?://[^\n]*)?\n?"
        )
        .expect(
            "ISF redundant iTime alias regex"
        );

    let without_time =
        time_alias.replace_all(
            without_resolution.as_ref(),
            ""
        );

    if without_time.as_ref()
        != without_resolution.as_ref()
    {
        applied.push(
            "ISF-COMP-0010:remove-redundant-iTime-alias"
                .to_string()
        );
    }

    without_time.into_owned()
}

fn initialize_main_output(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    let regex =
        Regex::new(
            r"void\s+main\s*\(\s*(?:void\s*)?\)\s*\{"
        )
        .expect(
            "ISF main function regex"
        );

    let Some(found) =
        regex.find(
            source
        )
    else {
        return source.to_string();
    };

    let nearby_end =
        (
            found.end() + 160
        )
        .min(
            source.len()
        );

    if source[
        found.end()
            ..nearby_end
    ]
    .contains(
        "fragColor = vec4(0.0)"
    ) {
        return source.to_string();
    }

    applied.push(
        "ISF-COMP-0004:initialize-main-output:fragColor"
            .to_string()
    );

    let mut output =
        String::with_capacity(
            source.len() + 32
        );

    output.push_str(
        &source[
            ..found.end()
        ]
    );

    output.push_str(
        "\n    fragColor = vec4(0.0);"
    );

    output.push_str(
        &source[
            found.end()..
        ]
    );

    output
}