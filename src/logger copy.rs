use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::File;

fn timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs() as i64;

    let tm = unsafe {
        let mut t: libc::tm = std::mem::zeroed();
        let time = secs as libc::time_t;
        libc::localtime_r(&time, &mut t);
        t
    };

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}

pub fn create_log(logfile: &Path) {
    if let Err(err) = File::create(logfile) {
        eprintln!(
            "[LOGGER] Unable to create {} ({})",
            logfile.display(),
            err
        );
    }
}

pub fn log(logfile: &Path, message: &str) {

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(logfile)
    {
        Ok(file) => file,

        Err(err) => {
            eprintln!(
                "LOGGER: Unable to open {} ({})",
                logfile.display(),
                err
            );
            return;
        }
    };

    let line = format!("{} {}\n", timestamp(), message);

    if let Err(err) = file.write_all(line.as_bytes()) {
        eprintln!("LOGGER: Write failed ({})", err);
        return;
    }

    if let Err(err) = file.flush() {
        eprintln!("LOGGER: Flush failed ({})", err);
    }
}
