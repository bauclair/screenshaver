//! Resolve a Control Center shader argument to either a file or directory.
//!
//! The legacy directory-preview execution path was removed with the public
//! `--preview-shader` command.  The Control Center still uses this module to
//! resolve an optional shader filename/path supplied to `--control`.

use std::path::PathBuf;


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
            crate::manage_localization::runtime_text_with_params(
                "preview.target_not_found",
                &[("path", &resolved.display().to_string())],
            )
        )
    }
}


pub enum PreviewTarget {
    File(PathBuf),
    Directory(PathBuf),
}
