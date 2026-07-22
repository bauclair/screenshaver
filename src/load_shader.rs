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

    let source_path =
        crate::locate_paths::shader_dir()
            .join(
                shader_name
            );


    load_shader_path_internal(
        &source_path,
        true,
    )
}


pub fn load_shader_for_preview(
    source_path: &Path,
) -> ShaderLoadResult {

    load_shader_path_internal(
        source_path,
        false,
    )
}


fn load_shader_path_internal(
    source_path: &Path,
    quarantine_on_rejection: bool,
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
                if quarantine_on_rejection {
                    quarantine(
                        source_path,
                        &shader_name,
                        &report.rejection_reasons,
                    );
                }
                return ShaderLoadResult::Rejected {
                    shader_name: shader_name.clone(),
                    reasons: report.rejection_reasons,
                };
            }

            let processed = if let Some(cached) = try_load_cached_shader(source_path) {
                cached
            } else {
                if let Err(error) = write_cached_shader(source_path, &report.source) {
                    log_warning(&format!("[CACHE] Failed to write cache entry: {error}"));
                }
                report.source
            };

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

            let processed =
                if let Some(cached) =
                    try_load_cached_shader(
                        source_path
                    )
                {
                    cached
                } else {

                    if let Err(error) =
                        write_cached_shader(
                            source_path,
                            &report.source,
                        )
                    {
                        log_warning(
                            &format!(
                                "[CACHE] Failed to write cache entry: {error}"
                            )
                        );
                    }

                    report.source
                };

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
                if quarantine_on_rejection {
                    quarantine(
                        source_path,
                        &shader_name,
                        &rejection_reasons,
                    );
                }
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

fn quarantine(source_path: &Path, shader_name: &str, reasons: &[String]) {
    match crate::reject_shader::reject_shader(source_path, reasons) {
        Ok(path) => log_information(&format!(
            "[REJECT] Shader '{}' quarantined at '{}'",
            shader_name,
            path.display()
        )),
        Err(error) => log_warning(&format!(
            "[REJECT] Failed to quarantine shader '{}': {}",
            source_path.display(),
            error
        )),
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

fn try_load_cached_shader(source_path: &Path) -> Option<String> {
    let cache_path = generated_cache_path(source_path);

    if !cache_path.exists() {
        log_debug(&format!("[CACHE] Miss: {}", cache_path.display()));
        return None;
    }

    if cache_is_stale(source_path, &cache_path) {
        log_debug(&format!("[CACHE] Stale: {}", cache_path.display()));
        return None;
    }

    match std::fs::read_to_string(&cache_path) {
        Ok(source) => {
            log_debug(&format!("[CACHE] Hit: {}", cache_path.display()));
            Some(source)
        }
        Err(error) => {
            log_warning(&format!(
                "[CACHE] Failed to read cache entry '{}': {}",
                cache_path.display(),
                error
            ));
            None
        }
    }
}

fn cache_is_stale(source_path: &Path, cache_path: &Path) -> bool {
    let source_modified = std::fs::metadata(source_path).and_then(|m| m.modified());
    let cache_modified = std::fs::metadata(cache_path).and_then(|m| m.modified());

    match (source_modified, cache_modified) {
        (Ok(source_time), Ok(cache_time)) => cache_time < source_time,
        _ => true,
    }
}

fn write_cached_shader(source_path: &Path, processed: &str) -> std::io::Result<PathBuf> {
    let cache_dir = crate::locate_paths::shader_cache_dir();
    std::fs::create_dir_all(&cache_dir)?;
    let output_path = generated_cache_path(source_path);
    std::fs::write(&output_path, processed)?;
    log_debug(&format!("[CACHE] Cache entry written: {}", output_path.display()));
    Ok(output_path)
}

fn generated_cache_path(source_path: &Path) -> PathBuf {
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("shader");

    crate::locate_paths::shader_cache_dir()
        .join(format!("{stem}._gen.glsl"))
}

pub fn cleanup_generated_shaders() {
    let directory = crate::locate_paths::shader_cache_dir();
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            log_warning(&format!(
                "[CACHE] Failed to read shader cache directory '{}': {}",
                directory.display(),
                error
            ));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if !name.contains("._gen") || !name.ends_with(".glsl") {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => log_debug(&format!("[CACHE] Deleted generated shader: {}", path.display())),
            Err(error) => log_warning(&format!(
                "[CACHE] Failed to delete generated shader '{}': {}",
                path.display(),
                error
            )),
        }
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

