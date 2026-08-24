use std::path::{Path, PathBuf};

const BUILTIN_DEFAULT_SHADER: &str = r#"
void mainImage(out vec4 fragColor, vec2 fragCoord)
{
    vec2 uv = fragCoord / iResolution.xy;
    float wave = 0.5 + 0.5 * sin(iTime + uv.x * 6.2831853 + uv.y * 3.1415926);
    vec3 color = vec3(uv.x, wave, 1.0 - uv.y);
    fragColor = vec4(color, 1.0);
}
"#;

#[derive(Debug)]
pub enum ShaderLoadResult {
    Ready {
        source: String,
        shader_name: String,
        built_in_default: bool,
        channel_usage:
            crate::preprocess_shader::ShaderChannelUsage,
        shader_inputs:
            Vec<crate::isf_types::ShaderInput>,
    },
    Rejected {
        shader_name: String,
        reasons: Vec<String>,
    },
    Unavailable {
        shader_name: String,
        error: String,
    },
}

pub fn load_shader(
    shader_name: &str,
) -> ShaderLoadResult {

    log_debug(
        &format!(
            "[SHADER] Attempting to load managed shader from database: {}",
            shader_name,
        )
    );


    match crate::load_shader_source::load_managed_shader_source(
        shader_name
    ) {

        Ok(
            crate::load_shader_source::ShaderSourceResult::Ready {
                source,
                shader_type,
                channel_usage,
                shader_inputs,
            }
        ) => {

            log_information(
                &format!(
                    "[SHADER] Successfully loaded managed shader from database: {}",
                    shader_name,
                )
            );


            log_debug(
                &format!(
                    "[SHADER] Runtime source: {} bytes",
                    source.len(),
                )
            );


            log_debug(
                &format!(
                    "[SHADER] Type: {}",
                    match shader_type.as_str() {
                        "native" => "Native GLSL",
                        "shadertoy" => "ShaderToy",
                        "isf" => "ISF",
                        other => other,
                    },
                )
            );


            ShaderLoadResult::Ready {
                source,
                shader_name:
                    shader_name.to_string(),
                built_in_default:
                    false,
                channel_usage,
                shader_inputs,
            }
        }


        Ok(
            crate::load_shader_source::ShaderSourceResult::Rejected {
                reason,
                message,
            }
        ) => {

            let mut reasons =
                Vec::<String>::new();


            if let Some(reason) =
                reason
            {
                reasons.push(
                    reason
                );
            }


            if let Some(message) =
                message
            {
                if !reasons.contains(
                    &message
                )
                {
                    reasons.push(
                        message
                    );
                }
            }


            if reasons.is_empty() {
                reasons.push(
                    "Shader is rejected by the Screenshaver database"
                        .to_string()
                );
            }


            log_warning(
                &format!(
                    "[SHADER] Managed shader '{}' is rejected: {}",
                    shader_name,
                    reasons.join(
                        "; "
                    ),
                )
            );


            ShaderLoadResult::Rejected {
                shader_name:
                    shader_name.to_string(),
                reasons,
            }
        }


        Ok(
            crate::load_shader_source::ShaderSourceResult::Unavailable {
                error,
            }
        ) => {

            log_error(
                &format!(
                    "[SHADER] Managed shader '{}' is unavailable: {}",
                    shader_name,
                    error,
                )
            );


            ShaderLoadResult::Unavailable {
                shader_name:
                    shader_name.to_string(),
                error,
            }
        }


        Err(error) => {

            log_error(
                &format!(
                    "[SHADER] Unable to load managed shader '{}' from database: {}",
                    shader_name,
                    error,
                )
            );


            ShaderLoadResult::Unavailable {
                shader_name:
                    shader_name.to_string(),
                error,
            }
        }
    }
}


pub fn load_shader_for_preview(
    source_path: &Path,
) -> ShaderLoadResult {

    load_shader_path_internal(
        source_path
    )
}


fn load_shader_path_internal(
    source_path: &Path,
) -> ShaderLoadResult {

    let shader_name =
        source_path
            .file_name()
            .and_then(
                |name| {
                    name.to_str()
                }
            )
            .unwrap_or(
                "<shader>"
            )
            .to_string();


    log_debug(
        &format!(
            "[SHADER] Attempting to load shader: {}",
            source_path.display(),
        )
    );


    let source = match std::fs::read_to_string(source_path) {
        Ok(source) => {
            log_information(&format!(
                "[SHADER] Successfully loaded shader: {}",
                source_path.display()
            ));
            source
        }
        Err(error) => {
            log_error(&format!(
                "[SHADER] Failed to load shader '{}': {}",
                source_path.display(),
                error
            ));
            return ShaderLoadResult::Unavailable {
                shader_name: shader_name.clone(),
                error: error.to_string(),
            };
        }
    };

    log_debug(&format!(
        "[SHADER] Loaded shader source: {} bytes",
        source.len()
    ));

    let shader_kind =
        crate::classify_shader::classify_shader(
            &source
        );


    match shader_kind {
        crate::classify_shader::ShaderKind::ShaderToy => {
            log_debug(
                "[SHADER] Type: ShaderToy"
            );


            let report = crate::preprocess_shader::preprocess_shader_with_report(&source);
            log_report(&report.applied, &report.warnings, &report.rejection_reasons);

            if !report.rejection_reasons.is_empty() {
                return ShaderLoadResult::Rejected {
                    shader_name: shader_name.clone(),
                    reasons: report.rejection_reasons,
                };
            }

            let processed = report.source;

            ShaderLoadResult::Ready {
                source: processed,
                shader_name: shader_name.clone(),
                built_in_default: false,
                channel_usage:
                    report.channel_usage,
                shader_inputs:
                    Vec::new(),
            }
        }

        crate::classify_shader::ShaderKind::Isf => {
            log_debug(
                "[SHADER] Type: ISF"
            );

            let document =
                match crate::parse_isf::parse(
                    &source
                ) {
                    Ok(document) => document,
                    Err(error) => {
                        log_error(
                            &format!(
                                "[ISF] Metadata parsing failed for '{}': {}",
                                source_path.display(),
                                error,
                            )
                        );

                        return ShaderLoadResult::Unavailable {
                            shader_name:
                                shader_name.clone(),
                            error:
                                format!(
                                    "ISF metadata parsing failed: {}",
                                    error,
                                ),
                        };
                    }
                };

            log_debug(
                &format!(
                    "[ISF] Version: {}",
                    document.metadata.version_name(),
                )
            );

            log_debug(
                &format!(
                    "[ISF] Inputs: {}",
                    document.metadata.inputs.len(),
                )
            );

            log_debug(
                &format!(
                    "[ISF] Passes: {}",
                    document.metadata.pass_count(),
                )
            );

            log_debug(
                &format!(
                    "[ISF] Imported resources: {}",
                    document.metadata.has_imported_resources(),
                )
            );

            let report =
                crate::preprocess_isf::preprocess(
                    &document
                );

            log_report(
                &report.applied,
                &report.warnings,
                &report.rejection_reasons,
            );

            if !report.rejection_reasons.is_empty() {
                log_warning(
                    "[ISF] Status: Unsupported"
                );

                return ShaderLoadResult::Rejected {
                    shader_name:
                        shader_name.clone(),
                    reasons:
                        report.rejection_reasons,
                };
            }

            log_information(
                "[ISF] Status: Supported"
            );

            let processed = report.source;

            ShaderLoadResult::Ready {
                source:
                    processed,
                shader_name:
                    shader_name.clone(),
                built_in_default:
                    false,
                channel_usage:
                    crate::preprocess_shader::ShaderChannelUsage::default(),
                shader_inputs:
                    report.inputs,
            }
        }

        crate::classify_shader::ShaderKind::NativeGLSL => {
            log_debug(
                "[SHADER] Type: Native GLSL"
            );


            let (warnings, rejection_reasons) =
                crate::preprocess_shader::analyze_native_shader(&source);

            let channel_usage =
                crate::preprocess_shader::analyze_native_channel_usage(
                    &source
                );

            log_report(&[], &warnings, &rejection_reasons);

            if !rejection_reasons.is_empty() {
                return ShaderLoadResult::Rejected {
                    shader_name: shader_name.clone(),
                    reasons: rejection_reasons,
                };
            }

            ShaderLoadResult::Ready {
                source,
                shader_name: shader_name.clone(),
                built_in_default: false,
                channel_usage,
                shader_inputs:
                    Vec::new(),
            }
        }
    }
}

pub fn load_builtin_default_shader() -> ShaderLoadResult {
    log_information("[SHADER] Loading built-in default shader");
    let report = crate::preprocess_shader::preprocess_shader_with_report(BUILTIN_DEFAULT_SHADER);

    if !report.rejection_reasons.is_empty() {
        log_error(
            &format!(
                "[SHADER] Built-in default shader failed validation: {}",
                report.rejection_reasons.join("; "),
            )
        );

        return ShaderLoadResult::Unavailable {
            shader_name: "<built-in-default>".to_string(),
            error: format!(
                "Built-in default shader failed validation: {}",
                report.rejection_reasons.join("; ")
            ),
        };
    }

    ShaderLoadResult::Ready {
        source: report.source,
        shader_name: "<built-in-default>".to_string(),
        built_in_default: true,
        channel_usage:
            report.channel_usage,
        shader_inputs:
            Vec::new(),
    }
}

fn log_report(applied: &[String], warnings: &[String], reasons: &[String]) {
    for item in applied {
        log_debug(&format!("[PREPROCESS] Applied: {item}"));
    }
    for item in warnings {
        log_warning(&format!("[PREPROCESS] Warning: {item}"));
    }
    for item in reasons {
        log_warning(&format!("[PREPROCESS] Rejection: {item}"));
    }
}

fn log_debug(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::debug(
        &logfile,
        message,
    );
}


fn log_information(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        message,
    );
}


fn log_warning(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::warning(
        &logfile,
        message,
    );
}


fn log_error(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::error(
        &logfile,
        message,
    );
}

