//! Discover and preview shader files in one directory.
//!
//! Directory scanning is deliberately non-recursive. Files are
//! sorted alphabetically before being passed to preview_shader.

use std::path::{
    Path,
    PathBuf,
};
use crate::parse_texture_specification::TextureSpecification;


const DEFAULT_DIRECTORY_INTERVAL_SECONDS: u64 =
    30;

pub fn run(
    directory_argument: String,
    shader_texture: Option<TextureSpecification>,
    shader_palette: Option<String>,
    interval_seconds: Option<u64>,
    fps: Option<u32>,
) -> Result<(), String> {

    let directory =
        resolve_directory_path(
            &directory_argument
        )?;

    let shader_paths =
        discover_shader_paths(
            &directory
        )?;

    if shader_paths.is_empty() {
        return Err(
            format!(
                "No .glsl, .fs, or .shaver shader files were found directly inside '{}'",
                directory.display(),
            )
        );
    }

    let interval_seconds =
        interval_seconds.unwrap_or(
            DEFAULT_DIRECTORY_INTERVAL_SECONDS
        );

    crate::preview_shader::run_paths(
        shader_paths,
        shader_texture,
        shader_palette,
        Some(interval_seconds),
        fps,
    )
}


pub fn resolve_preview_target(
    argument: &str,
) -> Result<
    PreviewTarget,
    String,
> {

    let supplied =
        PathBuf::from(
            argument
        );


    let resolved =
        if supplied.is_absolute()
            || supplied.components()
                .count()
                > 1
        {
            supplied
        } else {

            let local =
                crate::locate_paths::shader_dir()
                    .join(
                        &supplied
                    );


            if local.exists() {
                local
            } else {
                supplied
            }
        };


    if resolved.is_dir() {
        Ok(
            PreviewTarget::Directory(
                resolved
            )
        )

    } else if resolved.is_file() {

        Ok(
            PreviewTarget::File(
                resolved
            )
        )

    } else {

        Err(
            format!(
                "Shader file or directory not found: {}",
                resolved.display(),
            )
        )
    }
}


pub enum PreviewTarget {
    File(PathBuf),
    Directory(PathBuf),
}


fn resolve_directory_path(
    argument: &str,
) -> Result<PathBuf, String> {

    match resolve_preview_target(
        argument
    )? {

        PreviewTarget::Directory(
            path
        ) => {
            Ok(
                path
            )
        }

        PreviewTarget::File(
            path
        ) => {
            Err(
                format!(
                    "Expected a shader directory, but '{}' is a file",
                    path.display(),
                )
            )
        }
    }
}


fn discover_shader_paths(
    directory: &Path,
) -> Result<
    Vec<PathBuf>,
    String,
> {

    let entries =
        std::fs::read_dir(
            directory
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read shader directory '{}': {}",
                    directory.display(),
                    error,
                )
            }
        )?;


    let mut paths =
        Vec::new();


    for entry in
        entries
    {
        let entry =
            entry.map_err(
                |error| {
                    format!(
                        "Unable to read a shader directory entry: {}",
                        error,
                    )
                }
            )?;


        let path =
            entry.path();


        if !path.is_file() {
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


        if extension.eq_ignore_ascii_case(
            "glsl"
        )
            || extension.eq_ignore_ascii_case(
                "fs"
            )
            || extension.eq_ignore_ascii_case(
                "shaver"
            )
        {
            paths.push(
                path
            );
        }
    }


    paths.sort_by(
        |left, right| {
            let left_name =
                left.file_name()
                    .and_then(
                        |value| {
                            value.to_str()
                        }
                    )
                    .unwrap_or_default()
                    .to_ascii_lowercase();


            let right_name =
                right.file_name()
                    .and_then(
                        |value| {
                            value.to_str()
                        }
                    )
                    .unwrap_or_default()
                    .to_ascii_lowercase();


            left_name.cmp(
                &right_name
            )
        }
    );


    Ok(
        paths
    )
}

