//! Temporary non-locking renderer diagnostic state transport.
//! No database changes; updates are best-effort and never block presentation.
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Pipeline { Screensaver, Wallpaper, Windowshader, ControlCenter }
impl Pipeline {
    pub const ALL: [Self; 4] = [Self::Screensaver, Self::Wallpaper, Self::Windowshader, Self::ControlCenter];
    pub fn name(self) -> &'static str {
        match self { Self::Screensaver => "Screensaver", Self::Wallpaper => "Wallpaper", Self::Windowshader => "Windowshader", Self::ControlCenter => "Control Center" }
    }
    fn key(self) -> &'static str {
        match self { Self::Screensaver => "screensaver", Self::Wallpaper => "wallpaper", Self::Windowshader => "windowshader", Self::ControlCenter => "control-center" }
    }
}
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State { Starting, Rendering, Paused, Stopping, Error }
impl State {
    pub fn name(self) -> &'static str {
        match self { Self::Starting => "Starting", Self::Rendering => "Rendering", Self::Paused => "Paused", Self::Stopping => "Stopping", Self::Error => "Error" }
    }
}
fn base_dir() -> PathBuf {
    // Restrict the shared state to the current user's configuration directory.
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/screenshaver/.renderer-status")
}
/// The marker is owned by one live process, not by a persistent preference.
/// The Linux process start tick protects against PID reuse after a crash.
fn process_start_tick(pid: u32) -> Option<String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = stat.rsplit_once(") ")?.1;
    // /proc/PID/stat field 22; `after_comm` begins at field 3.
    after_comm.split_whitespace().nth(19).map(str::to_owned)
}

fn owner_token() -> Option<String> {
    let pid = std::process::id();
    Some(format!("{pid} {}", process_start_tick(pid)?))
}

fn active_owner() -> Option<String> {
    let marker = fs::read_to_string(base_dir().join("enabled")).ok()?;
    let mut fields = marker.split_whitespace();
    let pid = fields.next()?.parse::<u32>().ok()?;
    let start = fields.next()?;
    if process_start_tick(pid).as_deref() == Some(start) {
        Some(format!("{pid} {start}"))
    } else {
        None
    }
}

pub struct DiagnosticSession {
    owner: Option<String>,
}

impl Drop for DiagnosticSession {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.as_deref() {
            if active_owner().as_deref() == Some(owner) {
                let _ = fs::remove_file(base_dir().join("enabled"));
            }
        }
    }
}

/// Start a diagnostic session. `replace` is true only after resident singleton
/// acquisition, so a rejected second instance cannot change the live session.
pub fn begin_session(replace: bool) -> io::Result<DiagnosticSession> {
    let dir = base_dir();
    fs::create_dir_all(&dir)?;
    if !replace && active_owner().is_some() {
        return Ok(DiagnosticSession { owner: None });
    }
    let token = owner_token().ok_or_else(|| io::Error::new(
        io::ErrorKind::Other, "Unable to identify diagnostic owner process"
    ))?;
    let temp = dir.join(format!("enabled.{}.tmp", std::process::id()));
    fs::write(&temp, format!("{token}\n"))?;
    fs::rename(&temp, dir.join("enabled"))?;
    Ok(DiagnosticSession { owner: Some(token) })
}

/// Called by a newly acquired resident singleton without the diagnostic flag.
/// It also discards legacy persistent markers containing just `1`.
pub fn disable_for_new_runtime() {
    let _ = fs::remove_file(base_dir().join("enabled"));
}

pub fn enabled() -> bool { active_owner().is_some() }

fn millis() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}
/// Publish only after the corresponding renderer has confirmed a state change.
/// Failure is intentionally nonfatal: diagnostic I/O must not stop rendering.
pub fn publish(pipeline: Pipeline, state: State) {
    if !enabled() { return; }
    let dir = base_dir();
    let path = dir.join(pipeline.key());
    let temp = dir.join(format!("{}.{}.tmp", pipeline.key(), std::process::id()));
    let payload = format!("{} {} {}\n", std::process::id(), millis(), state.name());
    if fs::write(&temp, payload).is_ok() { let _ = fs::rename(&temp, path); }
}
/// Clear only the record owned by this process (avoid deleting a successor).
pub fn clear(pipeline: Pipeline) {
    if !enabled() { return; }
    let path = base_dir().join(pipeline.key());
    if let Ok(s) = fs::read_to_string(&path) {
        if s.split_whitespace().next() == Some(&std::process::id().to_string()[..]) {
            let _ = fs::remove_file(path);
        }
    }
}
/// Read the latest known state. Stale records are never described as Rendering.
pub fn snapshot(pipeline: Pipeline) -> &'static str {
    if !enabled() { return "Inactive"; }
    let s = match fs::read_to_string(base_dir().join(pipeline.key())) {
        Ok(s) => s, Err(_) => return "Inactive"
    };
    let mut fields = s.split_whitespace();
    let pid = fields.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
    let timestamp = fields.next().and_then(|x| x.parse::<u128>().ok()).unwrap_or(0);
    let state = fields.next().unwrap_or("Error");
    // A deliberately paused renderer has no frame heartbeat.  Preserve its
    // state while the owning process exists, rather than mislabeling it as
    // Inactive after five seconds.  Rendering still requires a fresh heartbeat.
    let age = millis().saturating_sub(timestamp);
    if state == "Paused" && age > Duration::from_secs(5).as_millis() {
        if pid == 0 || !PathBuf::from(format!("/proc/{pid}")).exists() {
            return "Inactive";
        }
    } else if age > Duration::from_secs(5).as_millis() {
        return "Inactive";
    }
    match state {
        "Starting" => "Starting", "Rendering" => "Rendering", "Paused" => "Paused",
        "Stopping" => "Stopping", "Error" => "Error", _ => "Error"
    }
}
pub fn panel_text() -> String {
    Pipeline::ALL.iter().map(|&p| format!("{}: {}", p.name(), snapshot(p))).collect::<Vec<_>>().join("\n")
}
