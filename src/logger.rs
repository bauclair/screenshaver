use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs() as i64;

    let tm = unsafe {
        let mut t: libc::tm = std::mem::zeroed();
        let time = secs as libc::time_t;

        if libc::localtime_r(&time, &mut t).is_null() {
            None
        } else {
            Some(t)
        }
    };

    match tm {
        Some(tm) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        ),

        None => format!("UNIX-{}", secs),
    }
}

fn ensure_parent_directory(logfile: &Path) -> bool {
    if let Some(parent) = logfile.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!(
                "[LOGGER] Unable to create directory {} ({})",
                parent.display(),
                err
            );
            return false;
        }
    }

    true
}

fn write_log_entry(logfile: &Path, message: &str) {
    if !ensure_parent_directory(logfile) {
        return;
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(logfile)
    {
        Ok(file) => file,

        Err(err) => {
            eprintln!(
                "[LOGGER] Unable to open {} ({})",
                logfile.display(),
                err
            );
            return;
        }
    };

    let line = format!("{} {}\n", timestamp(), message);

    if let Err(err) = file.write_all(line.as_bytes()) {
        eprintln!(
            "[LOGGER] Unable to write to {} ({})",
            logfile.display(),
            err
        );
        return;
    }

    if let Err(err) = file.flush() {
        eprintln!(
            "[LOGGER] Unable to flush {} ({})",
            logfile.display(),
            err
        );
    }
}

fn write_categorized_entry(
    logfile: &Path,
    level: u8,
    category: &str,
    message: &str,
) {
    write_log_entry(
        logfile,
        &format!("[L{}] [{}] {}", level, category, message),
    );
}

///
/// Create the log file if it does not already exist.
///
/// Existing contents are preserved.
///
pub fn ensure_log_exists(logfile: &Path) {
    if !ensure_parent_directory(logfile) {
        return;
    }

    if !logfile.exists() {
        if let Err(err) = File::create(logfile) {
            eprintln!(
                "[LOGGER] Unable to create log file {} ({})",
                logfile.display(),
                err
            );
        }
    }
}

///
/// Start a new logging session.
///
/// Any existing log is discarded.
///
pub fn reset_log(logfile: &Path) {
    if !ensure_parent_directory(logfile) {
        return;
    }

    if let Err(err) = File::create(logfile) {
        eprintln!(
            "[LOGGER] Unable to reset log file {} ({})",
            logfile.display(),
            err
        );
    }
}

///
/// Write an unclassified legacy log entry.
///
/// This function remains available during the logging migration so that
/// existing call sites continue to compile without modification.
///
//pub fn log(logfile: &Path, message: &str) {
//    write_log_entry(logfile, message);
//}

/// Write a Level 1 critical event.
pub fn critical(logfile: &Path, message: &str) {
    write_categorized_entry(logfile, 1, "CRITICAL", message);
}

/// Write a Level 2 error event.
pub fn error(logfile: &Path, message: &str) {
    write_categorized_entry(logfile, 2, "ERROR", message);
}

/// Write a Level 3 warning event.
pub fn warning(logfile: &Path, message: &str) {
    write_categorized_entry(logfile, 3, "WARNING", message);
}

/// Write a Level 4 informational event.
pub fn information(logfile: &Path, message: &str) {
    write_categorized_entry(logfile, 4, "INFORMATION", message);
}

/// Write a Level 5 debugging event.
pub fn debug(logfile: &Path, message: &str) {
    write_categorized_entry(logfile, 5, "DEBUG", message);
}

/// Write a Level 6 trace event.
pub fn trace(logfile: &Path, message: &str) {
    write_categorized_entry(logfile, 6, "TRACE", message);
}

