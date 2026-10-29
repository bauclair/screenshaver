//! MATE screensaver runtime ownership and reversible theme selection.
//! Native MATE remains responsible for authentication and unlocking.
//! This module is not activated until the resident lifecycle is wired up.
use std::{fs, io::{Read, Write}, os::unix::fs::{OpenOptionsExt, PermissionsExt}, path::{Path, PathBuf}, process::Command};

const SCHEMA: &str = "org.mate.screensaver";
const THEME: &str = "screensavers-screenshaver-mate";
const VERSION: &str = "mate-runtime-v1";

fn runtime_dir() -> Result<PathBuf, String> {
    let uid = fs::read_to_string("/proc/self/status").map_err(|e| e.to_string())?
        .lines().find(|l| l.starts_with("Uid:")).and_then(|l| l.split_whitespace().nth(2))
        .ok_or("Cannot determine effective UID")?.parse::<u32>().map_err(|e| e.to_string())?;
    let dir = PathBuf::from(format!("/run/user/{uid}/screenshaver"));
    Ok(dir)
}
fn config_dir() -> Result<PathBuf, String> {
    let root = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(p) if Path::new(&p).is_absolute() => PathBuf::from(p),
        _ => PathBuf::from(std::env::var_os("HOME").ok_or("HOME missing")?).join(".config"),
    };
    Ok(root.join("screenshaver"))
}
fn marker_path() -> Result<PathBuf, String> { Ok(runtime_dir()?.join("mate-runtime-owner")) }
fn snapshot_path() -> Result<PathBuf, String> { Ok(config_dir()?.join("mate-saver-runtime.restore")) }
fn start_ticks(pid: u32) -> Result<Option<u64>, String> {
    let s = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let end = s.rfind(')').ok_or("Invalid process stat")?;
    Ok(Some(s[end+1..].split_whitespace().nth(19).ok_or("Missing process start time")?
        .parse::<u64>().map_err(|e| e.to_string())?))
}
fn read_marker(path: &Path) -> Result<Option<(u32,u64)>, String> {
    let s = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut it = s.lines();
    if it.next() != Some(VERSION) { return Err("Invalid MATE ownership marker version".into()); }
    let pid = it.next().ok_or("Missing owner PID")?.parse::<u32>().map_err(|e| e.to_string())?;
    let ticks = it.next().ok_or("Missing owner start time")?.parse::<u64>().map_err(|e| e.to_string())?;
    if pid == 0 || it.next().is_some() { return Err("Malformed MATE ownership marker".into()); }
    Ok(Some((pid,ticks)))
}
fn owner_live(owner: (u32,u64)) -> Result<bool,String> {
    Ok(start_ticks(owner.0)? == Some(owner.1))
}
pub(crate) fn resident_runtime_active() -> Result<bool,String> {
    match read_marker(&marker_path()?)? { Some(owner) => owner_live(owner), None => Ok(false) }
}
fn get(key: &str) -> Result<String,String> {
    let out = Command::new("gsettings").args(["get",SCHEMA,key]).output().map_err(|e| e.to_string())?;
    if !out.status.success() { return Err(format!("MATE gsettings get {key}: {}", String::from_utf8_lossy(&out.stderr))); }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
fn set(key: &str, value: &str) -> Result<(),String> {
    let out = Command::new("gsettings").args(["set",SCHEMA,key,value]).output().map_err(|e| e.to_string())?;
    if !out.status.success() { return Err(format!("MATE gsettings set {key}: {}", String::from_utf8_lossy(&out.stderr))); }
    Ok(())
}
#[derive(Debug, Clone)]
struct Settings { mode: String, themes: String }
fn current() -> Result<Settings,String> { Ok(Settings { mode: get("mode")?, themes: get("themes")? }) }
fn restore(s: &Settings) -> Result<(),String> { set("themes", &s.themes)?; set("mode", &s.mode) }
fn write_private(path: &Path, data: &str) -> Result<(),String> {
    let parent = path.parent().ok_or("Missing parent directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    fs::set_permissions(parent,fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    let mut f = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp).map_err(|e| e.to_string())?;
    let result = (|| { f.write_all(data.as_bytes()).map_err(|e| e.to_string())?; f.sync_all().map_err(|e| e.to_string())?; fs::rename(&tmp,path).map_err(|e| e.to_string()) })();
    if result.is_err() { let _ = fs::remove_file(&tmp); }
    result
}
fn read_snapshot(path: &Path) -> Result<Settings,String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut s = String::new(); f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    let mut lines = s.lines();
    if lines.next() != Some(VERSION) { return Err("Invalid MATE recovery snapshot".into()); }
    let mode = lines.next().ok_or("Missing saved MATE mode")?.to_string();
    let themes = lines.next().ok_or("Missing saved MATE themes")?.to_string();
    if lines.next().is_some() { return Err("Malformed MATE recovery snapshot".into()); }
    Ok(Settings {mode,themes})
}
fn selected() -> Result<bool,String> {
    Ok(get("mode")? == "'single'" && get("themes")? == format!("['{THEME}']"))
}
fn recover_stale(path: &Path) -> Result<(),String> {
    if !path.exists() { return Ok(()); }
    let saved = read_snapshot(path)?;
    if selected()? { restore(&saved)?; }
    fs::remove_file(path).map_err(|e| e.to_string())
}
#[derive(Debug)]
pub(crate) struct MateRuntimeSession {
    marker: PathBuf, snapshot: PathBuf, owner: (u32,u64), previous: Settings, active: bool,
}
impl MateRuntimeSession {
    /// Call only after the resident process owns the application's singleton.
    pub(crate) fn acquire(_logfile: &Path) -> Result<Self,String> {
        let marker = marker_path()?;
        if let Some(owner) = read_marker(&marker)? {
            if owner_live(owner)? { return Err("Another live MATE runtime owner exists".into()); }
            fs::remove_file(&marker).map_err(|e| e.to_string())?;
        }
        let snapshot = snapshot_path()?;
        recover_stale(&snapshot)?;
        let previous = current()?;
        if previous.mode.contains(['\n','\r']) || previous.themes.contains(['\n','\r']) {
            return Err("Unexpected multiline MATE settings".into());
        }
        write_private(&snapshot, &format!("{VERSION}\n{}\n{}\n",previous.mode,previous.themes))?;
        let pid = std::process::id();
        let owner = (pid,start_ticks(pid)?.ok_or("Current process not found")?);
        if let Err(e) = write_private(&marker,&format!("{VERSION}\n{}\n{}\n",owner.0,owner.1)) {
            let _ = fs::remove_file(&snapshot); return Err(e);
        }
        if let Err(e) = set("themes",&format!("['{THEME}']")).and_then(|_|set("mode","single")) {
            if restore(&previous).is_ok() { let _ = fs::remove_file(&snapshot); }
            let _ = fs::remove_file(&marker);
            return Err(e);
        }
        Ok(Self {marker,snapshot,owner,previous,active:true})
    }
    fn release(&mut self) {
        if !self.active { return; }
        if matches!(read_marker(&self.marker), Ok(Some(owner)) if owner == self.owner) {
            if restore(&self.previous).is_ok() { let _ = fs::remove_file(&self.snapshot); }
            let _ = fs::remove_file(&self.marker);
        }
        self.active = false;
    }
}
impl Drop for MateRuntimeSession { fn drop(&mut self) { self.release(); } }
