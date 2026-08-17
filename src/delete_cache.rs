//! Manage Screenshaver's cached preprocessed shader files.
//!
//! Cache deletion is initiated by the Control Center. Only regular files
//! directly inside
//! ~/.config/screenshaver/cache. Subdirectories and their contents
//! are deliberately left untouched.

use std::fs;
use std::path::{
    Path,
    PathBuf,
};


#[derive(Debug, Clone)]
pub struct DeleteCacheResult {
    pub deleted_count: usize,
}


pub fn count_cache_files() -> Result<usize, String> {

    let cache_dir =
        crate::locate_paths::shader_cache_dir();


    if !cache_dir.exists() {
        return Ok(0);
    }


    if !cache_dir.is_dir() {
        return Err(
            format!(
                "Cache path is not a directory: {}",
                cache_dir.display(),
            )
        );
    }


    Ok(
        collect_cache_files(
            &cache_dir
        )?
        .len()
    )
}


pub fn delete_cache_files() -> Result<DeleteCacheResult, String> {

    let cache_dir =
        crate::locate_paths::shader_cache_dir();


    let logging_enabled =
        debug_logging_enabled();


    let logfile =
        crate::locate_paths::runtime_log_path();


    if !cache_dir.exists() {

        if logging_enabled {
            crate::logger::debug(
                &logfile,
                &format!(
                    "[CACHE] Cache directory does not exist: {}",
                    cache_dir.display(),
                ),
            );
        }


        return Ok(
            DeleteCacheResult {
                deleted_count:
                    0,
            }
        );
    }


    if !cache_dir.is_dir() {

        return Err(
            format!(
                "Cache path is not a directory: {}",
                cache_dir.display(),
            )
        );
    }


    let cache_files =
        collect_cache_files(
            &cache_dir
        )?;


    if cache_files.is_empty() {

        if logging_enabled {
            crate::logger::debug(
                &logfile,
                "[CACHE] Cache already empty",
            );
        }


        return Ok(
            DeleteCacheResult {
                deleted_count:
                    0,
            }
        );
    }


    let mut deleted_count =
        0_usize;


    for path in
        cache_files
    {
        fs::remove_file(
            &path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to delete cache file '{}': {}",
                    path.display(),
                    error,
                )
            }
        )?;


        deleted_count +=
            1;


        if logging_enabled {
            let display_name =
                path.file_name()
                    .and_then(
                        |name| {
                            name.to_str()
                        }
                    )
                    .map(
                        str::to_string
                    )
                    .unwrap_or_else(
                        || {
                            path.display()
                                .to_string()
                        }
                    );


            crate::logger::debug(
                &logfile,
                &format!(
                    "[CACHE] Deleted: {}",
                    display_name,
                ),
            );
        }
    }


    if logging_enabled {
        crate::logger::information(
            &logfile,
            &format!(
                "[CACHE] Deleted {} cache {}",
                deleted_count,
                if deleted_count == 1 {
                    "file"
                } else {
                    "files"
                },
            ),
        );
    }


    Ok(
        DeleteCacheResult {
            deleted_count,
        }
    )
}


fn collect_cache_files(
    cache_dir: &Path,
) -> Result<
    Vec<PathBuf>,
    String,
> {

    let entries =
        fs::read_dir(
            cache_dir
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


    let mut files =
        Vec::new();


    for entry in
        entries
    {
        let entry =
            entry.map_err(
                |error| {
                    format!(
                        "Unable to read a cache directory entry: {}",
                        error,
                    )
                }
            )?;


        let file_type =
            entry.file_type()
                .map_err(
                    |error| {
                        format!(
                            "Unable to inspect cache entry '{}': {}",
                            entry.path().display(),
                            error,
                        )
                    }
                )?;


        if file_type.is_file() {
            files.push(
                entry.path()
            );
        }
    }


    files.sort_by(
        |left, right| {
            left.file_name()
                .cmp(
                    &right.file_name()
                )
        }
    );


    Ok(
        files
    )
}


fn debug_logging_enabled() -> bool {

    crate::load_config::load_config(
        &crate::locate_paths::config_path()
    )
    .map(
        |result| {
            result.config.debug_log
        }
    )
    .unwrap_or(
        false
    )
}

