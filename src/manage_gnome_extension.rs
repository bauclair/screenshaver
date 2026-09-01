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
use std::process;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;


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
