use std::fs;
use std::io;
use std::path::{
    Path,
    PathBuf,
};


pub fn wallpaper_directory() -> io::Result<PathBuf> {

    Ok(
        crate::initialize_user_files::config_directory()?
            .join(
                "wallpapers"
            )
    )
}


pub fn locate_shader_files(
    directory: &Path,
) -> io::Result<Vec<PathBuf>> {

    let mut shader_files =
        Vec::new();


    for entry in
        fs::read_dir(
            directory
        )?
    {
        let entry =
            entry?;


        let file_type =
            entry.file_type()?;


        if !file_type.is_file() {
            continue;
        }


        let path =
            entry.path();


        if is_supported_shader_file(
            &path
        ) {
            shader_files.push(
                path
            );
        }
    }


    shader_files.sort_by(
        |left, right| {

            let left_name =
                left
                    .file_name()
                    .and_then(
                        |name| {
                            name.to_str()
                        }
                    )
                    .unwrap_or_default()
                    .to_ascii_lowercase();


            let right_name =
                right
                    .file_name()
                    .and_then(
                        |name| {
                            name.to_str()
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
        shader_files
    )
}


fn is_supported_shader_file(
    path: &Path,
) -> bool {

    let Some(extension) =
        path.extension()
            .and_then(
                |extension| {
                    extension.to_str()
                }
            )
    else {
        return false;
    };


    matches!(
        extension
            .to_ascii_lowercase()
            .as_str(),

        "glsl"
            | "fs"
            | "shaver"
    )
}

