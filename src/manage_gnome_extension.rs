use std::fs::{
    self,
    File,
    OpenOptions,
};
use std::io::{
    Read,
    Write,
};
use std::path::{
    Path,
    PathBuf,
};
use std::process::{
    self,
    Command,
};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;



const GNOME_EXTENSION_UUID: &str =
    "screenshaver@screenshaver";

const GNOME_EXTENSION_JS: &str =
    include_str!(
        "../assets/gnome-extension/extension.js"
    );

const GNOME_EXTENSION_METADATA: &str =
    include_str!(
        "../assets/gnome-extension/metadata.json"
    );

const EXTENSION_STATE_VERIFY_ATTEMPTS: usize =
    10;

const EXTENSION_STATE_VERIFY_DELAY:
    Duration =
    Duration::from_millis(
        100
    );


#[derive(Debug)]
pub(crate) struct GnomeExtensionIntegrationGuard {
    logfile: PathBuf,
    enabled: bool,
}


impl GnomeExtensionIntegrationGuard {
    /// Provision Screenshaver's GNOME Shell extension into the current user's
    /// extension directory, reload it when GNOME already knows about the UUID,
    /// enable it, and verify that GNOME Shell reports it ACTIVE.
    ///
    /// Installation is persistent; activation is runtime-owned.  The extension
    /// itself remains fail-closed and creates no Screenshaver actors unless the
    /// runtime marker and matching shared-memory session handshake are valid.
    pub(crate) fn activate(
        logfile: &Path,
    ) -> Result<Self, String> {

        let extension_dir =
            extension_directory()?;


        fs::create_dir_all(
            &extension_dir
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create GNOME extension directory '{}': {}",
                    extension_dir.display(),
                    error,
                )
            }
        )?;


        set_directory_permissions(
            &extension_dir
        )?;


        let extension_changed =
            write_asset_if_changed(
                &extension_dir.join(
                    "extension.js"
                ),
                GNOME_EXTENSION_JS.as_bytes(),
            )?;

        let metadata_changed =
            write_asset_if_changed(
                &extension_dir.join(
                    "metadata.json"
                ),
                GNOME_EXTENSION_METADATA.as_bytes(),
            )?;


        if extension_changed
            || metadata_changed
        {
            crate::logger::information(
                logfile,
                &format!(
                    "[LOCK] GNOME extension assets provisioned at '{}'",
                    extension_dir.display(),
                ),
            );
        } else {
            crate::logger::information(
                logfile,
                &format!(
                    "[LOCK] GNOME extension assets already current at '{}'",
                    extension_dir.display(),
                ),
            );
        }


        // If the extension is already known in this GNOME Shell session,
        // disabling before enabling forces Shell to load the provisioned copy.
        // Failure here is intentionally non-fatal because a first installation
        // may not yet be known to GNOME Shell.
        let _ =
            run_gnome_extensions_command(
                &[
                    "disable",
                    GNOME_EXTENSION_UUID,
                ]
            );


        let enable_output =
            run_gnome_extensions_command(
                &[
                    "enable",
                    GNOME_EXTENSION_UUID,
                ]
            )
            .map_err(
                |error| {
                    format!(
                        "{} If this is the extension's first installation in the current GNOME login session, log out and back in once so GNOME Shell can discover it.",
                        error,
                    )
                }
            )?;


        if !enable_output.status.success() {
            return Err(
                format!(
                    "Unable to enable GNOME extension '{}': {}. If this is the extension's first installation in the current GNOME login session, log out and back in once so GNOME Shell can discover it.",
                    GNOME_EXTENSION_UUID,
                    command_output_message(
                        &enable_output
                    ),
                )
            );
        }


        if !wait_for_extension_active()? {
            return Err(
                format!(
                    "GNOME extension '{}' was enabled but did not reach ACTIVE state. The stock GNOME lock screen will be used. If the extension was newly installed, log out and back in once.",
                    GNOME_EXTENSION_UUID,
                )
            );
        }


        crate::logger::information(
            logfile,
            "[LOCK] Screenshaver GNOME Shell extension enabled and ACTIVE",
        );


        Ok(
            Self {
                logfile:
                    logfile.to_path_buf(),

                enabled:
                    true,
            }
        )
    }


    pub(crate) fn deactivate(
        &mut self,
    ) -> Result<(), String> {

        if !self.enabled {
            return Ok(());
        }


        let disable_output =
            run_gnome_extensions_command(
                &[
                    "disable",
                    GNOME_EXTENSION_UUID,
                ]
            )?;


        if !disable_output.status.success() {
            return Err(
                format!(
                    "Unable to disable GNOME extension '{}': {}",
                    GNOME_EXTENSION_UUID,
                    command_output_message(
                        &disable_output
                    ),
                )
            );
        }


        if !wait_for_extension_inactive()? {
            return Err(
                format!(
                    "GNOME extension '{}' did not leave ACTIVE state after disable request",
                    GNOME_EXTENSION_UUID,
                )
            );
        }


        self.enabled =
            false;


        crate::logger::information(
            &self.logfile,
            "[LOCK] Screenshaver GNOME Shell extension disabled",
        );


        Ok(())
    }
}


impl Drop for GnomeExtensionIntegrationGuard {
    fn drop(
        &mut self,
    ) {

        if !self.enabled {
            return;
        }


        if let Err(error) =
            self.deactivate()
        {
            crate::logger::warning(
                &self.logfile,
                &format!(
                    "[LOCK] Unable to disable Screenshaver GNOME Shell extension during cleanup: {}",
                    error,
                ),
            );
        }
    }
}


/// Best-effort stale-state cleanup for a GNOME Screenshaver runtime that is
/// running with secure screen locking disabled.  Persistent extension files are
/// intentionally retained; only GNOME Shell activation is withdrawn.
pub(crate) fn disable_if_present(
    logfile: &Path,
) {

    let output =
        match run_gnome_extensions_command(
            &[
                "disable",
                GNOME_EXTENSION_UUID,
            ]
        ) {
            Ok(output) => {
                output
            }

            Err(error) => {
                crate::logger::information(
                    logfile,
                    &format!(
                        "[LOCK] GNOME extension stale-state cleanup skipped: {}",
                        error,
                    ),
                );

                return;
            }
        };


    if output.status.success() {
        crate::logger::information(
            logfile,
            "[LOCK] Screenshaver GNOME Shell extension is inactive because screen locking is disabled",
        );
    }
}


fn extension_directory()
    -> Result<PathBuf, String>
{

    let data_home =
        if let Some(data_home) =
            std::env::var_os(
                "XDG_DATA_HOME"
            )
        {
            PathBuf::from(
                data_home
            )
        } else {
            let home =
                std::env::var_os(
                    "HOME"
                )
                .map(
                    PathBuf::from
                )
                .ok_or_else(
                    || {
                        "Neither XDG_DATA_HOME nor HOME is available for GNOME extension provisioning"
                            .to_string()
                    }
                )?;

            home
                .join(
                    ".local"
                )
                .join(
                    "share"
                )
        };


    Ok(
        data_home
            .join(
                "gnome-shell"
            )
            .join(
                "extensions"
            )
            .join(
                GNOME_EXTENSION_UUID
            )
    )
}


fn write_asset_if_changed(
    path: &Path,
    contents: &[u8],
) -> Result<bool, String> {

    match fs::read(
        path
    ) {
        Ok(existing)
            if existing == contents =>
        {
            return Ok(
                false
            );
        }

        Ok(_) => {}

        Err(error)
            if error.kind()
                == std::io::ErrorKind::NotFound =>
        {}

        Err(error) => {
            return Err(
                format!(
                    "Unable to read GNOME extension asset '{}': {}",
                    path.display(),
                    error,
                )
            );
        }
    }


    let file_name =
        path.file_name()
            .and_then(
                |name| {
                    name.to_str()
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "GNOME extension asset path '{}' has no valid file name",
                        path.display(),
                    )
                }
            )?;


    let temporary_path =
        path.with_file_name(
            format!(
                ".{}.screenshaver.tmp.{}",
                file_name,
                process::id(),
            )
        );


    let write_result =
        (|| -> Result<(), String> {

            let mut options =
                OpenOptions::new();

            options
                .write(true)
                .create_new(true);


            #[cfg(unix)]
            {
                options.mode(
                    0o644
                );
            }


            let mut file =
                options
                    .open(
                        &temporary_path
                    )
                    .map_err(
                        |error| {
                            format!(
                                "Unable to create temporary GNOME extension asset '{}': {}",
                                temporary_path.display(),
                                error,
                            )
                        }
                    )?;


            file.write_all(
                contents
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to write temporary GNOME extension asset '{}': {}",
                        temporary_path.display(),
                        error,
                    )
                }
            )?;


            file.sync_all()
                .map_err(
                    |error| {
                        format!(
                            "Unable to synchronize temporary GNOME extension asset '{}': {}",
                            temporary_path.display(),
                            error,
                        )
                    }
                )?;


            drop(
                file
            );


            set_file_permissions(
                &temporary_path
            )?;


            fs::rename(
                &temporary_path,
                path,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to install GNOME extension asset '{}' as '{}': {}",
                        temporary_path.display(),
                        path.display(),
                        error,
                    )
                }
            )?;


            Ok(())
        })();


    if write_result.is_err() {
        let _ =
            fs::remove_file(
                &temporary_path
            );
    }


    write_result?;


    Ok(
        true
    )
}


#[cfg(unix)]
fn set_directory_permissions(
    path: &Path,
) -> Result<(), String> {

    use std::os::unix::fs::PermissionsExt;


    fs::set_permissions(
        path,
        fs::Permissions::from_mode(
            0o755
        ),
    )
    .map_err(
        |error| {
            format!(
                "Unable to set GNOME extension directory permissions on '{}': {}",
                path.display(),
                error,
            )
        }
    )
}


#[cfg(not(unix))]
fn set_directory_permissions(
    _path: &Path,
) -> Result<(), String> {

    Ok(())
}


#[cfg(unix)]
fn set_file_permissions(
    path: &Path,
) -> Result<(), String> {

    use std::os::unix::fs::PermissionsExt;


    fs::set_permissions(
        path,
        fs::Permissions::from_mode(
            0o644
        ),
    )
    .map_err(
        |error| {
            format!(
                "Unable to set GNOME extension asset permissions on '{}': {}",
                path.display(),
                error,
            )
        }
    )
}


#[cfg(not(unix))]
fn set_file_permissions(
    _path: &Path,
) -> Result<(), String> {

    Ok(())
}


fn run_gnome_extensions_command(
    arguments: &[&str],
) -> Result<std::process::Output, String> {

    Command::new(
        "gnome-extensions"
    )
    .args(
        arguments
    )
    .output()
    .map_err(
        |error| {
            format!(
                "Unable to execute 'gnome-extensions {}': {}",
                arguments.join(
                    " "
                ),
                error,
            )
        }
    )
}


fn command_output_message(
    output: &std::process::Output,
) -> String {

    let stderr =
        String::from_utf8_lossy(
            &output.stderr
        )
        .trim()
        .to_string();

    if !stderr.is_empty() {
        return stderr;
    }


    let stdout =
        String::from_utf8_lossy(
            &output.stdout
        )
        .trim()
        .to_string();

    if !stdout.is_empty() {
        return stdout;
    }


    format!(
        "command exited with status {}",
        output.status,
    )
}


fn extension_state()
    -> Result<Option<String>, String>
{

    let output =
        run_gnome_extensions_command(
            &[
                "info",
                GNOME_EXTENSION_UUID,
            ]
        )?;


    if !output.status.success() {
        return Ok(
            None
        );
    }


    let stdout =
        String::from_utf8_lossy(
            &output.stdout
        );


    for line in stdout.lines() {
        let trimmed =
            line.trim();

        if let Some(value) =
            trimmed.strip_prefix(
                "State:"
            )
        {
            return Ok(
                Some(
                    value.trim()
                        .to_ascii_uppercase()
                )
            );
        }
    }


    Ok(
        None
    )
}


fn wait_for_extension_active()
    -> Result<bool, String>
{

    for _ in 0..EXTENSION_STATE_VERIFY_ATTEMPTS {
        if extension_state()?
            .as_deref()
            == Some(
                "ACTIVE"
            )
        {
            return Ok(
                true
            );
        }


        thread::sleep(
            EXTENSION_STATE_VERIFY_DELAY
        );
    }


    Ok(
        false
    )
}


fn wait_for_extension_inactive()
    -> Result<bool, String>
{

    for _ in 0..EXTENSION_STATE_VERIFY_ATTEMPTS {
        match extension_state()? {
            Some(state)
                if state == "ACTIVE" =>
            {}

            _ => {
                return Ok(
                    true
                );
            }
        }


        thread::sleep(
            EXTENSION_STATE_VERIFY_DELAY
        );
    }


    Ok(
        false
    )
}


const RUNTIME_MARKER_FILENAME: &str =
    "screenshaver-gnome-lock.active";

const RUNTIME_MARKER_VERSION: u32 =
    1;

const SESSION_ID_BYTES: usize =
    16;


#[derive(Debug)]
pub(crate) struct GnomeRuntimeSession {
    logfile: PathBuf,
    marker_path: PathBuf,
    pid: u32,
    session_id: String,
    active: bool,
}


impl GnomeRuntimeSession {
    /// Establish ownership of Screenshaver's GNOME lock-screen integration for
    /// the lifetime of the resident Screenshaver process.
    ///
    /// The marker alone never authorizes the GNOME extension to display
    /// Screenshaver visuals.  The production extension will additionally
    /// require the matching shared-memory presentation transport.
    pub(crate) fn acquire(
        logfile: &Path,
    ) -> Result<Self, String> {

        let runtime_dir =
            std::env::var_os(
                "XDG_RUNTIME_DIR"
            )
            .map(
                PathBuf::from
            )
            .ok_or_else(
                || {
                    "XDG_RUNTIME_DIR is unavailable for GNOME runtime ownership"
                        .to_string()
                }
            )?;


        let marker_path =
            runtime_dir.join(
                RUNTIME_MARKER_FILENAME
            );


        remove_stale_marker_if_safe(
            &marker_path,
            logfile,
        )?;


        let pid =
            process::id();

        let session_id =
            generate_session_id()?;


        write_marker(
            &marker_path,
            pid,
            &session_id,
        )?;


        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] GNOME runtime ownership established: pid={} marker={}",
                pid,
                marker_path.display(),
            ),
        );


        Ok(
            Self {
                logfile:
                    logfile.to_path_buf(),

                marker_path,
                pid,
                session_id,
                active:
                    true,
            }
        )
    }


    pub(crate) fn session_id(
        &self,
    ) -> &str {

        &self.session_id
    }


    pub(crate) fn marker_path(
        &self,
    ) -> &Path {

        &self.marker_path
    }


    pub(crate) fn pid(
        &self,
    ) -> u32 {

        self.pid
    }


    /// Remove the ownership marker only when it still belongs to this exact
    /// Screenshaver runtime.  A newer or foreign marker is never removed.
    pub(crate) fn release(
        &mut self,
    ) {

        if !self.active {
            return;
        }


        match marker_matches(
            &self.marker_path,
            self.pid,
            &self.session_id,
        ) {
            Ok(true) => {

                match fs::remove_file(
                    &self.marker_path
                ) {
                    Ok(()) => {

                        crate::logger::information(
                            &self.logfile,
                            "[LOCK] GNOME runtime ownership released",
                        );
                    }

                    Err(error)
                        if error.kind()
                            == std::io::ErrorKind::NotFound =>
                    {
                        crate::logger::information(
                            &self.logfile,
                            "[LOCK] GNOME runtime ownership marker was already absent during cleanup",
                        );
                    }

                    Err(error) => {

                        crate::logger::warning(
                            &self.logfile,
                            &format!(
                                "[LOCK] Unable to remove GNOME runtime ownership marker '{}': {}",
                                self.marker_path.display(),
                                error,
                            ),
                        );
                    }
                }
            }


            Ok(false) => {

                crate::logger::warning(
                    &self.logfile,
                    &format!(
                        "[LOCK] GNOME runtime ownership marker '{}' no longer belongs to this Screenshaver runtime; it was not removed",
                        self.marker_path.display(),
                    ),
                );
            }


            Err(error) => {

                crate::logger::warning(
                    &self.logfile,
                    &format!(
                        "[LOCK] Unable to verify GNOME runtime ownership marker during cleanup: {}",
                        error,
                    ),
                );
            }
        }


        self.active =
            false;
    }
}


impl Drop for GnomeRuntimeSession {
    fn drop(
        &mut self,
    ) {

        self.release();
    }
}


fn generate_session_id() -> Result<String, String> {

    let mut random =
        File::open(
            "/dev/urandom"
        )
        .map_err(
            |error| {
                format!(
                    "Unable to open /dev/urandom for GNOME runtime session identity: {}",
                    error,
                )
            }
        )?;


    let mut bytes =
        [0u8; SESSION_ID_BYTES];


    random
        .read_exact(
            &mut bytes
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read GNOME runtime session identity from /dev/urandom: {}",
                    error,
                )
            }
        )?;


    let mut session_id =
        String::with_capacity(
            SESSION_ID_BYTES * 2
        );


    for byte in bytes {
        use std::fmt::Write as _;

        write!(
            &mut session_id,
            "{byte:02x}"
        )
        .map_err(
            |error| {
                format!(
                    "Unable to format GNOME runtime session identity: {}",
                    error,
                )
            }
        )?;
    }


    Ok(
        session_id
    )
}


fn write_marker(
    marker_path: &Path,
    pid: u32,
    session_id: &str,
) -> Result<(), String> {

    let mut options =
        OpenOptions::new();

    options
        .write(true)
        .create_new(true);


    #[cfg(unix)]
    options.mode(
        0o600
    );


    let mut file =
        options
            .open(
                marker_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create GNOME runtime ownership marker '{}': {}",
                        marker_path.display(),
                        error,
                    )
                }
            )?;


    let contents =
        format!(
            "version={}\npid={}\nsession_id={}\n",
            RUNTIME_MARKER_VERSION,
            pid,
            session_id,
        );


    if let Err(error) =
        file.write_all(
            contents.as_bytes()
        )
    {
        let _ =
            fs::remove_file(
                marker_path
            );

        return Err(
            format!(
                "Unable to write GNOME runtime ownership marker '{}': {}",
                marker_path.display(),
                error,
            )
        );
    }


    if let Err(error) =
        file.sync_all()
    {
        let _ =
            fs::remove_file(
                marker_path
            );

        return Err(
            format!(
                "Unable to synchronize GNOME runtime ownership marker '{}': {}",
                marker_path.display(),
                error,
            )
        );
    }


    Ok(())
}


fn remove_stale_marker_if_safe(
    marker_path: &Path,
    logfile: &Path,
) -> Result<(), String> {

    let contents =
        match fs::read_to_string(
            marker_path
        ) {
            Ok(contents) => {
                contents
            }

            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {
                return Ok(());
            }

            Err(error) => {
                return Err(
                    format!(
                        "Unable to inspect existing GNOME runtime ownership marker '{}': {}",
                        marker_path.display(),
                        error,
                    )
                );
            }
        };


    let marker =
        parse_marker(
            &contents
        )
        .map_err(
            |error| {
                format!(
                    "Existing GNOME runtime ownership marker '{}' is invalid: {}",
                    marker_path.display(),
                    error,
                )
            }
        )?;


    if process_exists(
        marker.pid
    ) {
        return Err(
            format!(
                "GNOME runtime ownership marker '{}' belongs to live process {}; refusing to replace it",
                marker_path.display(),
                marker.pid,
            )
        );
    }


    fs::remove_file(
        marker_path
    )
    .map_err(
        |error| {
            format!(
                "Unable to remove stale GNOME runtime ownership marker '{}': {}",
                marker_path.display(),
                error,
            )
        }
    )?;


    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] Removed stale GNOME runtime ownership marker for process {}",
            marker.pid,
        ),
    );


    Ok(())
}


fn marker_matches(
    marker_path: &Path,
    expected_pid: u32,
    expected_session_id: &str,
) -> Result<bool, String> {

    let contents =
        match fs::read_to_string(
            marker_path
        ) {
            Ok(contents) => {
                contents
            }

            Err(error)
                if error.kind()
                    == std::io::ErrorKind::NotFound =>
            {
                return Ok(false);
            }

            Err(error) => {
                return Err(
                    format!(
                        "Unable to read GNOME runtime ownership marker '{}': {}",
                        marker_path.display(),
                        error,
                    )
                );
            }
        };


    let marker =
        parse_marker(
            &contents
        )?;


    Ok(
        marker.pid
            == expected_pid
            && marker.session_id
                == expected_session_id
    )
}


struct ParsedMarker {
    pid: u32,
    session_id: String,
}


fn parse_marker(
    contents: &str,
) -> Result<ParsedMarker, String> {

    let mut version =
        None;

    let mut pid =
        None;

    let mut session_id =
        None;


    for line in contents.lines() {
        let Some(
            (
                key,
                value,
            )
        ) =
            line.split_once('=')
        else {
            continue;
        };


        match key {
            "version" => {

                version =
                    Some(
                        value
                            .parse::<u32>()
                            .map_err(
                                |error| {
                                    format!(
                                        "invalid version '{}': {}",
                                        value,
                                        error,
                                    )
                                }
                            )?
                    );
            }


            "pid" => {

                pid =
                    Some(
                        value
                            .parse::<u32>()
                            .map_err(
                                |error| {
                                    format!(
                                        "invalid pid '{}': {}",
                                        value,
                                        error,
                                    )
                                }
                            )?
                    );
            }


            "session_id" => {

                session_id =
                    Some(
                        value.to_string()
                    );
            }


            _ => {}
        }
    }


    let version =
        version
            .ok_or_else(
                || {
                    "missing version"
                        .to_string()
                }
            )?;


    if version
        != RUNTIME_MARKER_VERSION
    {
        return Err(
            format!(
                "unsupported version {}",
                version,
            )
        );
    }


    let pid =
        pid
            .ok_or_else(
                || {
                    "missing pid"
                        .to_string()
                }
            )?;


    let session_id =
        session_id
            .ok_or_else(
                || {
                    "missing session_id"
                        .to_string()
                }
            )?;


    if session_id.len()
        != SESSION_ID_BYTES * 2
        || !session_id
            .bytes()
            .all(
                |byte| {
                    byte.is_ascii_hexdigit()
                }
            )
    {
        return Err(
            "invalid session_id"
                .to_string()
        );
    }


    Ok(
        ParsedMarker {
            pid,
            session_id,
        }
    )
}


fn process_exists(
    pid: u32,
) -> bool {

    Path::new(
        "/proc"
    )
    .join(
        pid.to_string()
    )
    .exists()
}
