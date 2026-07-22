//! Delete Screenshaver's cached preprocessed shader files.
//!
//! This maintenance command removes regular files directly inside
//! ~/.config/screenshaver/cache. Subdirectories and their contents
//! are deliberately left untouched.

use std::fs;
use std::path::{
    Path,
    PathBuf,
};


pub fn run() -> Result<(), String> {

    let cache_dir =
        crate::locate_paths::shader_cache_dir();


    let logging_enabled =
        debug_logging_enabled();


    let logfile =
        crate::locate_paths::runtime_log_path();


    if logging_enabled {
        crate::logger::information(
            &logfile,
            "[CACHE] Delete requested from command line",
        );
    }


    if !cache_dir.exists() {

        println!(
            "Cache directory does not exist: {}",
            cache_dir.display(),
        );


        if logging_enabled {
            crate::logger::debug(
                &logfile,
                &format!(
                    "[CACHE] Cache directory does not exist: {}",
                    cache_dir.display(),
                ),
            );
        }


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


    let cache_files =
        collect_cache_files(
            &cache_dir
        )?;


    if cache_files.is_empty() {

        println!(
            "Cache is already empty."
        );


        if logging_enabled {
            crate::logger::debug(
                &logfile,
                "[CACHE] Cache already empty",
            );
        }


        return Ok(());
    }


    let mut deleted_files =
        Vec::with_capacity(
            cache_files.len()
        );


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


        if logging_enabled {
            crate::logger::debug(
                &logfile,
                &format!(
                    "[CACHE] Deleted: {}",
                    display_name,
                ),
            );
        }


        deleted_files.push(
            display_name
        );
    }


    println!(
        "Deleted:"
    );


    for filename in
        &deleted_files
    {
        println!(
            "    {}",
            filename
        );
    }


    println!();


    let count =
        deleted_files.len();


    println!(
        "Deleted {} cached shader {}.",
        count,
        if count == 1 {
            "file"
        } else {
            "files"
        },
    );


    if logging_enabled {
        crate::logger::information(
            &logfile,
            &format!(
                "[CACHE] Deleted {} cache {}",
                count,
                if count == 1 {
                    "file"
                } else {
                    "files"
                },
            ),
        );
    }


    Ok(())
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

