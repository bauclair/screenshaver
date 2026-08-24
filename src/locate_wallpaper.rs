use std::io;
use std::path::{
    Path,
    PathBuf,
};


pub fn wallpaper_directory() -> io::Result<PathBuf> {

    Ok(
        crate::locate_paths::shader_dir()
    )
}


pub fn locate_shader_files(
    directory: &Path,
) -> io::Result<Vec<PathBuf>> {

    let source_path =
        directory
            .to_string_lossy()
            .to_string();


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Unable to open database for wallpaper shader discovery: {}",
                            error
                        ),
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT filename
                 FROM shaders
                 WHERE source_path = ?1
                   AND file_status = 'present'
                 ORDER BY filename COLLATE NOCASE,
                          filename"
            )
            .map_err(
                |error| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Unable to prepare wallpaper shader discovery query: {}",
                            error
                        ),
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                rusqlite::params![
                    source_path
                ],
                |row| {
                    row.get::<_, String>(0)
                },
            )
            .map_err(
                |error| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Unable to query wallpaper shader discovery rows: {}",
                            error
                        ),
                    )
                }
            )?;


    let mut shader_files =
        Vec::new();


    for row in rows {

        let filename =
            row.map_err(
                |error| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Unable to decode wallpaper shader discovery row: {}",
                            error
                        ),
                    )
                }
            )?;


        let path =
            directory.join(
                filename
            );


        if is_supported_shader_file(
            &path
        ) {
            shader_files.push(
                path
            );
        }
    }


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
