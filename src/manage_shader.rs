use std::fs;


/// Determines how shaders are selected.
#[derive(Debug, Clone)]
pub enum ShaderMode {

    Single(
        String
    ),

    Random,

    Ordered,
}


/// Manages shader discovery and selection.
#[derive(Clone)]
pub struct ShaderManager {

    shaders:
        Vec<String>,

    index:
        usize,

    mode:
        ShaderMode,
}


impl ShaderManager {

    /// Create a new shader manager and load the available shader list.
    pub fn new(
        mode: ShaderMode,
    ) -> Self {

        Self::from_shader_list(
            mode,
            Self::load_shader_list(),
        )
    }


    /// Create a shader manager from an existing shader-name list.
    pub fn from_shader_list(
        mode: ShaderMode,
        mut shaders: Vec<String>,
    ) -> Self {

        shaders.sort();


        shaders.dedup();


        if shaders.is_empty() {

            log_warning(
                "[SHADER] No user shaders found"
            );
        }


        Self {

            shaders,

            index:
                0,

            mode,
        }
    }


    /// Scan the shader directory.
    pub fn load_shader_list() -> Vec<String> {

        let directory =
            crate::locate_paths::shader_dir();


        let entries =
            match fs::read_dir(
                &directory
            ) {

                Ok(entries) => entries,


                Err(error) => {

                    log_error(
                        &format!(
                            "[SHADER] Cannot read shader directory '{}': {}",
                            directory.display(),
                            error,
                        )
                    );


                    return Vec::new();
                }
            };


        let mut shaders =
            Vec::new();


        for entry in
            entries.flatten()
        {
            let path =
                entry.path();


            if !path.is_file() {

                continue;
            }


            let file_name =
                match path.file_name()
                    .and_then(
                        |name| name.to_str()
                    )
                {

                    Some(name) => name,

                    None => continue,
                };


            let extension =
                path.extension()
                    .and_then(
                        |value| {
                            value.to_str()
                        }
                    )
                    .unwrap_or_default();


            if !extension.eq_ignore_ascii_case(
                "glsl"
            )
                && !extension.eq_ignore_ascii_case(
                    "fs"
                )
                && !extension.eq_ignore_ascii_case(
                    "shaver"
                )
            {
                continue;
            }


            if file_name.contains(
                "._gen"
            ) {
                continue;
            }


            log_debug(
                &format!(
                    "[SHADER] Discovered shader: {}",
                    file_name
                )
            );


            shaders.push(
                file_name.to_string()
            );
        }


        shaders.sort();


        shaders
    }


    pub fn shader_count(
        &self,
    ) -> usize {

        self.shaders.len()
    }


    pub fn remove_shader(
        &mut self,
        shader_name: &str,
    ) {

        self.shaders.retain(
            |name| {

                name
                    != shader_name
            }
        );


        if self.shaders.is_empty() {

            self.index =
                0;

        } else if self.index
            >= self.shaders.len()
        {
            self.index %=
                self.shaders.len();
        }


        log_information(
            &format!(
                "[SHADER] Removed rejected shader from active list: {}",
                shader_name
            )
        );
    }


    /// Return the next shader according to the configured mode.
    pub fn next(
        &mut self,
    ) -> Option<String> {

        match &self.mode {

            ShaderMode::Single(
                name
            ) => {

                if self.shaders.contains(
                    name
                ) {
                    Some(
                        name.clone()
                    )

                } else {

                    log_warning(
                        &format!(
                            "[SHADER] Requested shader '{}' is unavailable; selecting another shader",
                            name
                        )
                    );


                    self.random_shader()
                }
            }


            ShaderMode::Random => {

                self.random_shader()
            }


            ShaderMode::Ordered => {

                self.ordered_shader()
            }
        }
    }


    fn random_shader(
        &self,
    ) -> Option<String> {

        use std::time::{
            SystemTime,
            UNIX_EPOCH,
        };


        let length =
            self.shaders.len();


        if length == 0 {

            return None;
        }


        let seed =
            SystemTime::now()
                .duration_since(
                    UNIX_EPOCH
                )
                .unwrap_or_default()
                .as_nanos()
                as usize;


        Some(
            self.shaders[
                seed % length
            ]
            .clone()
        )
    }


    fn ordered_shader(
        &mut self,
    ) -> Option<String> {

        let length =
            self.shaders.len();


        if length == 0 {

            return None;
        }


        let shader =
            self.shaders[
                self.index % length
            ]
            .clone();


        self.index =
            (
                self.index + 1
            )
            % length;


        Some(
            shader
        )
    }
}


//
// ------------------------------------------------------------
// Structured logging helpers
// ------------------------------------------------------------
//

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

