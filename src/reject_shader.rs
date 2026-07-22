use std::path::{
    Path,
    PathBuf,
};


//
// ------------------------------------------------------------
// Move a rejected shader out of the active shader directory
// ------------------------------------------------------------
//

pub fn reject_shader(
    source_path: &Path,
    reasons: &[String],
) -> Result<PathBuf, String> {

    let rejected_dir =
        crate::locate_paths::rejected_shader_dir();


    std::fs::create_dir_all(
        &rejected_dir
    )
    .map_err(
        |error| {

            format!(
                "Unable to create rejected shader directory '{}': {}",
                rejected_dir.display(),
                error,
            )
        }
    )?;


    let file_name =
        source_path
            .file_name()
            .and_then(
                |name| name.to_str()
            )
            .ok_or_else(
                || {

                    format!(
                        "Shader path has no valid filename: {}",
                        source_path.display()
                    )
                }
            )?;


    let rejected_path =
        unique_rejected_path(
            &rejected_dir,
            file_name,
        );


    move_file(
        source_path,
        &rejected_path,
    )?;


    write_reason_file(
        &rejected_path,
        reasons,
    )?;


    remove_cached_shader(
        source_path
    );


    log_information(
        &format!(
            "[REJECT] Shader moved from '{}' to '{}'",
            source_path.display(),
            rejected_path.display(),
        )
    );


    for reason in reasons {

        log_warning(
            &format!(
                "[REJECT] Reason: {}",
                reason
            )
        );
    }


    Ok(
        rejected_path
    )
}


//
// ------------------------------------------------------------
// Avoid overwriting a previously rejected shader
// ------------------------------------------------------------
//

fn unique_rejected_path(
    directory: &Path,
    file_name: &str,
) -> PathBuf {

    let first_path =
        directory.join(
            file_name
        );


    if !first_path.exists() {

        return first_path;
    }


    let original =
        Path::new(
            file_name
        );


    let stem =
        original
            .file_stem()
            .and_then(
                |value| value.to_str()
            )
            .unwrap_or(
                "shader"
            );


    let extension =
        original
            .extension()
            .and_then(
                |value| value.to_str()
            )
            .unwrap_or(
                "glsl"
            );


    for index in
        1_u32..
    {
        let candidate =
            directory.join(
                format!(
                    "{}.rejected-{}.{}",
                    stem,
                    index,
                    extension,
                )
            );


        if !candidate.exists() {

            return candidate;
        }
    }


    unreachable!()
}


//
// ------------------------------------------------------------
// Rename, with copy/remove fallback
// ------------------------------------------------------------
//

fn move_file(
    source: &Path,
    destination: &Path,
) -> Result<(), String> {

    match std::fs::rename(
        source,
        destination,
    ) {

        Ok(()) => {

            Ok(())
        }


        Err(rename_error) => {

            std::fs::copy(
                source,
                destination,
            )
            .map_err(
                |copy_error| {

                    format!(
                        "Unable to move rejected shader '{}' to '{}'. Rename error: {}. Copy error: {}",
                        source.display(),
                        destination.display(),
                        rename_error,
                        copy_error,
                    )
                }
            )?;


            std::fs::remove_file(
                source
            )
            .map_err(
                |remove_error| {

                    format!(
                        "Rejected shader was copied to '{}', but the original '{}' could not be removed: {}",
                        destination.display(),
                        source.display(),
                        remove_error,
                    )
                }
            )?;


            Ok(())
        }
    }
}


//
// ------------------------------------------------------------
// Write a human-readable rejection report
// ------------------------------------------------------------
//

fn write_reason_file(
    rejected_path: &Path,
    reasons: &[String],
) -> Result<(), String> {

    let reason_path =
        PathBuf::from(
            format!(
                "{}.reason.txt",
                rejected_path.display()
            )
        );


    let mut report =
        String::from(
            "Screenshaver rejected this shader.\n\nReasons:\n"
        );


    for reason in reasons {

        report.push_str(
            &format!(
                "- {}\n",
                reason
            )
        );
    }


    std::fs::write(
        &reason_path,
        report,
    )
    .map_err(
        |error| {

            format!(
                "Unable to write rejection report '{}': {}",
                reason_path.display(),
                error,
            )
        }
    )
}


//
// ------------------------------------------------------------
// Remove cache entries associated with the rejected shader
// ------------------------------------------------------------
//

fn remove_cached_shader(
    source_path: &Path,
) {

    let Some(stem) =
        source_path
            .file_stem()
            .and_then(
                |value| value.to_str()
            )
    else {
        return;
    };


    let cache_dir =
        crate::locate_paths::shader_cache_dir();


    let entries =
        match std::fs::read_dir(
            &cache_dir
        ) {

            Ok(entries) => entries,

            Err(_) => return,
        };


    for entry in
        entries.flatten()
    {
        let path =
            entry.path();


        let Some(name) =
            path.file_name()
                .and_then(
                    |value| value.to_str()
                )
        else {
            continue;
        };


        let belongs_to_shader =
            name.starts_with(
                &format!(
                    "{}._gen",
                    stem
                )
            )
            && name.ends_with(
                ".glsl"
            );


        if belongs_to_shader {

            match std::fs::remove_file(
                &path
            ) {

                Ok(()) => {

                    log_debug(
                        &format!(
                            "[CACHE] Removed rejected shader cache entry: {}",
                            path.display()
                        )
                    );
                }


                Err(error) => {

                    log_warning(
                        &format!(
                            "[CACHE] Failed to remove rejected shader cache entry '{}': {}",
                            path.display(),
                            error,
                        )
                    );
                }
            }
        }
    }
}


//
// ------------------------------------------------------------
// Logging
// ------------------------------------------------------------
//

fn log_debug(message: &str) {
    let logfile = crate::locate_paths::runtime_log_path();
    crate::logger::debug(&logfile, message);
}

fn log_information(message: &str) {
    let logfile = crate::locate_paths::runtime_log_path();
    crate::logger::information(&logfile, message);
}

fn log_warning(message: &str) {
    let logfile = crate::locate_paths::runtime_log_path();
    crate::logger::warning(&logfile, message);
}