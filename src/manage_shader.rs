use std::path::PathBuf;


/// Determines how shaders are selected.
#[derive(Debug, Clone)]
pub enum ShaderMode {

    Single(
        String
    ),

    Random,

    Ordered,
}


/// A selectable shader identified by its policy/display filename and,
/// when known, its resolved physical source path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderEntry {

    /// SQLite policy identity. Zero is reserved for compatibility-only
    /// synthetic entries that were not loaded from the policy database.
    pub policy_id:
        i64,

    /// Physical/logical shader filename used for loading and configured
    /// Single-mode matching.
    pub name:
        String,

    /// User-facing Policy Name. This is independent of the shader filename.
    pub policy_name:
        String,

    pub source_path:
        Option<PathBuf>,
}


impl ShaderEntry {

    pub fn named(
        name: String,
    ) -> Self {

        Self {
            policy_id:
                0,
            policy_name:
                name.clone(),
            name,
            source_path:
                None,
        }
    }


    pub fn with_source_path(
        name: String,
        source_path: PathBuf,
    ) -> Self {

        Self {
            policy_id:
                0,
            policy_name:
                name.clone(),
            name,
            source_path:
                Some(source_path),
        }
    }


    pub fn with_policy_source_path(
        name: String,
        policy_name: String,
        source_path: PathBuf,
    ) -> Self {

        Self {
            policy_id:
                0,
            name,
            policy_name,
            source_path:
                Some(source_path),
        }
    }


    pub fn with_policy_id_source_path(
        policy_id: i64,
        name: String,
        policy_name: String,
        source_path: PathBuf,
    ) -> Self {

        Self {
            policy_id,
            name,
            policy_name,
            source_path:
                Some(source_path),
        }
    }
}


/// Manages shader discovery and selection.
#[derive(Clone)]
pub struct ShaderManager {

    shaders:
        Vec<ShaderEntry>,

    index:
        usize,

    mode:
        ShaderMode,

    resume_shader:
        Option<String>,
}


impl ShaderManager {

    /// Create a new shader manager and load the available shader list.
    pub fn new(
        mode: ShaderMode,
    ) -> Self {

        Self::from_shader_entries(
            mode,
            Self::load_shader_entries(),
        )
    }


    /// Create a shader manager that presents one requested shader first, then
    /// continues using the configured selection mode.
    pub fn new_with_initial_shader(
        mode: ShaderMode,
        initial_shader: String,
    ) -> Self {

        let mut manager =
            Self::from_shader_entries(
                mode,
                Self::load_shader_entries(),
            );


        if manager.shaders.iter()
            .any(
                |shader| {
                    shader.name
                        == initial_shader
                }
            )
        {

            if matches!(
                manager.mode,
                ShaderMode::Ordered
            ) {
                if let Some(position) =
                    manager.shaders.iter()
                        .position(
                            |shader| {
                                shader.name
                                    == initial_shader
                            }
                        )
                {
                    manager.index =
                        if manager.shaders.is_empty() {
                            0
                        } else {
                            (position + 1)
                                % manager.shaders.len()
                        };
                }
            }

            manager.resume_shader =
                Some(initial_shader);

        } else {
            log_warning(
                &format!(
                    "[SHADER] Requested resume shader '{}' is unavailable; continuing with configured selection mode",
                    initial_shader
                )
            );
        }


        manager
    }


    /// Compatibility constructor for callers that currently provide only
    /// logical shader names.  These entries do not yet carry explicit paths.
    pub fn from_shader_list(
        mode: ShaderMode,
        shaders: Vec<String>,
    ) -> Self {

        let entries =
            shaders
                .into_iter()
                .map(
                    ShaderEntry::named
                )
                .collect();

        Self::from_shader_entries(
            mode,
            entries,
        )
    }


    /// Create a shader manager from path-aware shader entries.
    pub fn from_shader_entries(
        mode: ShaderMode,
        mut shaders: Vec<ShaderEntry>,
    ) -> Self {

        shaders.sort_by(
            |left, right| {
                left.name
                    .cmp(
                        &right.name
                    )
                    .then_with(
                        || {
                            left.policy_name
                                .cmp(
                                    &right.policy_name
                                )
                        }
                    )
                    .then_with(
                        || {
                            left.policy_id.cmp(
                                &right.policy_id
                            )
                        }
                    )
                    .then_with(
                        || {
                            left.source_path
                                .cmp(
                                    &right.source_path
                                )
                        }
                    )
            }
        );


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

            resume_shader:
                None,
        }
    }


    /// Load screensaver-eligible managed shaders from SQLite and preserve each
    /// shader's registered physical source path. A managed shader is selectable
    /// only when it has at least one Screensaver-target policy. Physical directory
    /// enumeration belongs to reconciliation, not normal runtime selection.
    pub fn load_shader_entries() -> Vec<ShaderEntry> {

        let managed_source_path =
            crate::locate_paths::shader_dir()
                .to_string_lossy()
                .to_string();


        let mut shaders =
            Vec::new();


        match crate::open_database::open() {

            Ok(connection) => {

                match connection.prepare(
                    "SELECT
                         p.policy_id,
                         s.filename,
                         s.source_path,
                         p.policy_name
                     FROM shader_policies AS p
                     JOIN shaders AS s
                       ON s.shader_id = p.shader_id
                     WHERE s.source_path = ?1
                       AND s.file_status = 'present'
                       AND p.policy_target = 'screensaver'
                     ORDER BY s.filename COLLATE NOCASE,
                              s.filename,
                              p.policy_name COLLATE NOCASE,
                              p.policy_name,
                              p.policy_id"
                ) {

                    Ok(mut statement) => {

                        match statement.query_map(
                            rusqlite::params![
                                managed_source_path
                            ],
                            |row| {
                                Ok(
                                    (
                                        row.get::<_, i64>(0)?,
                                        row.get::<_, String>(1)?,
                                        row.get::<_, String>(2)?,
                                        row.get::<_, String>(3)?,
                                    )
                                )
                            },
                        ) {

                            Ok(rows) => {

                                for row in rows {

                                    match row {

                                        Ok((
                                            policy_id,
                                            filename,
                                            source_path,
                                            policy_name,
                                        )) => {

                                            log_debug(
                                                &format!(
                                                    "[SHADER] Discovered screensaver policy '{}' for managed shader '{}'",
                                                    policy_name,
                                                    filename
                                                )
                                            );


                                            let physical_path =
                                                PathBuf::from(
                                                    &source_path
                                                )
                                                .join(
                                                    &filename
                                                );


                                            shaders.push(
                                                ShaderEntry::with_policy_id_source_path(
                                                    policy_id,
                                                    filename,
                                                    policy_name,
                                                    physical_path,
                                                )
                                            );
                                        }


                                        Err(error) => {

                                            log_warning(
                                                &format!(
                                                    "[SHADER] Unable to decode managed shader discovery row: {}",
                                                    error,
                                                )
                                            );
                                        }
                                    }
                                }
                            }


                            Err(error) => {

                                log_error(
                                    &format!(
                                        "[SHADER] Unable to query managed shader discovery rows: {}",
                                        error,
                                    )
                                );
                            }
                        }
                    }


                    Err(error) => {

                        log_error(
                            &format!(
                                "[SHADER] Unable to prepare managed shader discovery query: {}",
                                error,
                            )
                        );
                    }
                }
            }


            Err(error) => {

                log_error(
                    &format!(
                        "[SHADER] Unable to open database for managed shader discovery: {}",
                        error,
                    )
                );
            }
        }


        let config_path =
            crate::locate_paths::config_path();

        match crate::manage_policies::external_policy_entries(
            &config_path,
            crate::manage_policies::PolicyTarget::Screensaver,
        ) {
            Ok(external_paths) => {
                for (
                    policy_id,
                    policy_name,
                    name,
                    source_path,
                ) in external_paths
                {
                    if !source_path.is_file() {
                        log_warning(
                            &format!(
                                "[SHADER] External screensaver shader '{}' is unavailable: {}",
                                name,
                                source_path.display(),
                            )
                        );

                        continue;
                    }

                    shaders.push(
                        ShaderEntry::with_policy_id_source_path(
                            policy_id,
                            name,
                            policy_name,
                            source_path,
                        )
                    );
                }
            }

            Err(error) => {
                log_warning(
                    &format!(
                        "[SHADER] Unable to load external screensaver shader paths: {}",
                        error,
                    )
                );
            }
        }


        shaders.sort_by(
            |left, right| {
                left.name
                    .cmp(
                        &right.name
                    )
                    .then_with(
                        || {
                            left.policy_name
                                .cmp(
                                    &right.policy_name
                                )
                        }
                    )
                    .then_with(
                        || {
                            left.policy_id.cmp(
                                &right.policy_id
                            )
                        }
                    )
                    .then_with(
                        || {
                            left.source_path
                                .cmp(
                                    &right.source_path
                                )
                        }
                    )
            }
        );


        if shaders.is_empty() {

            log_warning(
                "[SHADER] No selectable shaders found"
            );
        }


        shaders
    }


    /// Compatibility helper returning only logical shader names.
    pub fn load_shader_list() -> Vec<String> {

        Self::load_shader_entries()
            .into_iter()
            .map(
                |shader| shader.name
            )
            .collect()
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
            |shader| {

                shader.name
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


    pub fn remove_entry(
        &mut self,
        entry: &ShaderEntry,
    ) {

        self.shaders.retain(
            |shader| {
                shader != entry
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
                "[SHADER] Removed rejected policy entry from active list: policy_id={}, shader={}",
                entry.policy_id,
                entry.name,
            )
        );
    }


    /// Return the next path-aware shader entry according to the configured
    /// mode.
    pub fn next_entry(
        &mut self,
    ) -> Option<ShaderEntry> {

        if let Some(shader_name) =
            self.resume_shader.take()
        {
            return self.shaders
                .iter()
                .find(
                    |shader| {
                        shader.name
                            == shader_name
                    }
                )
                .cloned();
        }


        match &self.mode {

            ShaderMode::Single(
                name
            ) => {

                if let Some(shader) =
                    self.shaders
                        .iter()
                        .find(
                            |shader| {
                                shader.name
                                    == *name
                            }
                        )
                {
                    Some(
                        shader.clone()
                    )

                } else {

                    log_warning(
                        &format!(
                            "[SHADER] Requested shader '{}' is unavailable; selecting another shader",
                            name
                        )
                    );


                    self.random_shader_entry()
                }
            }


            ShaderMode::Random => {

                self.random_shader_entry()
            }


            ShaderMode::Ordered => {

                self.ordered_shader_entry()
            }
        }
    }


    /// Compatibility selector returning only the logical shader name.
    pub fn next(
        &mut self,
    ) -> Option<String> {

        self.next_entry()
            .map(
                |shader| shader.name
            )
    }


    fn random_shader_entry(
        &self,
    ) -> Option<ShaderEntry> {

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


    fn ordered_shader_entry(
        &mut self,
    ) -> Option<ShaderEntry> {

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

