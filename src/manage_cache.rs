use sha2::{
    Digest,
    Sha256,
};

use std::collections::HashSet;
use std::path::{
    Path,
    PathBuf,
};


pub fn cache_key(
    source_path: &Path,
    source: &[u8],
) -> Result<String, String> {

    let canonical_path =
        std::fs::canonicalize(
            source_path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to canonicalize shader path '{}': {}",
                    source_path.display(),
                    error,
                )
            }
        )?;


    let mut hasher =
        Sha256::new();


    hasher.update(
        canonical_path
            .as_os_str()
            .as_encoded_bytes()
    );

    hasher.update(
        [0]
    );

    hasher.update(
        source
    );


    Ok(
        format!(
            "{:x}",
            hasher.finalize()
        )
    )
}


pub fn cache_path(
    source_path: &Path,
    source: &[u8],
) -> Result<PathBuf, String> {

    let key =
        cache_key(
            source_path,
            source,
        )?;


    Ok(
        crate::locate_paths::shader_cache_dir()
            .join(
                key
            )
    )
}


/// Dry-run garbage collection for normal Screenshaver startup.
/// No files are deleted by this checkpoint.
pub fn report_stale_cache_entries(
) -> Result<(), String> {

    let valid_keys =
        current_valid_cache_keys()?;

    let cache_dir =
        crate::locate_paths::shader_cache_dir();

    if !cache_dir.exists() {
        log_debug(
            "[CACHE] Garbage-collection dry run: cache directory does not exist"
        );
        return Ok(());
    }

    if !cache_dir.is_dir() {
        return Err(
            format!(
                "Cache path is not a directory: {}",
                cache_dir.display(),
            )
        );
    }

    let entries =
        std::fs::read_dir(
            &cache_dir
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read cache directory '{}': {}",
                    cache_dir.display(),
                    error,
                )
            }
        )?;

    let mut cache_objects = 0usize;
    let mut stale_objects = 0usize;

    for entry in entries {
        let entry = entry.map_err(
            |error| {
                format!(
                    "Unable to read a cache directory entry: {}",
                    error,
                )
            }
        )?;

        let file_type = entry.file_type().map_err(
            |error| {
                format!(
                    "Unable to inspect cache entry '{}': {}",
                    entry.path().display(),
                    error,
                )
            }
        )?;

        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if !is_managed_cache_object_name(name) {
            continue;
        }

        cache_objects += 1;

        if !valid_keys.contains(name) {
            stale_objects += 1;
            log_information(
                &format!(
                    "[CACHE] Garbage-collection dry run: would delete stale cache object {}",
                    name,
                )
            );
        }
    }

    log_information(
        &format!(
            "[CACHE] Garbage-collection dry run complete: {} cache objects inspected, {} stale",
            cache_objects,
            stale_objects,
        )
    );

    Ok(())
}


fn current_valid_cache_keys(
) -> Result<HashSet<String>, String> {

    let mut source_paths = Vec::<PathBuf>::new();

    for shader in crate::manage_shader::ShaderManager::load_shader_entries() {
        if let Some(source_path) = shader.source_path {
            source_paths.push(source_path);
        }
    }

    for shader in crate::manage_wallpaper::load_shader_entries()? {
        if let Some(source_path) = shader.source_path {
            source_paths.push(source_path);
        }
    }

    source_paths.sort();
    source_paths.dedup();

    let mut valid_keys = HashSet::new();

    for source_path in source_paths {
        if !source_path.is_file() {
            continue;
        }

        let source = std::fs::read(&source_path).map_err(
            |error| {
                format!(
                    "Unable to read shader '{}' while calculating cache reachability: {}",
                    source_path.display(),
                    error,
                )
            }
        )?;

        valid_keys.insert(
            cache_key(
                &source_path,
                &source,
            )?
        );
    }

    Ok(valid_keys)
}


fn is_managed_cache_object_name(
    name: &str,
) -> bool {

    name.len() == 64
        && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}


fn log_debug(
    message: &str,
) {

    let logfile = crate::locate_paths::runtime_log_path();
    crate::logger::debug(&logfile, message);
}


fn log_information(
    message: &str,
) {

    let logfile = crate::locate_paths::runtime_log_path();
    crate::logger::information(&logfile, message);
}

