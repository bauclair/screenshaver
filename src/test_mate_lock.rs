//! Temporary native MATE/X11 lock presentation test.
//! Authentication and input isolation remain owned by mate-screensaver.
use std::{fs, path::PathBuf, process::{Command, Stdio}, thread, time::{Duration, Instant}};

const SCHEMA: &str = "org.mate.screensaver";
const THEME: &str = "screensavers-screenshaver-mate-test";
const DEST: &str = "org.mate.ScreenSaver";
const OBJECT: &str = "/org/mate/ScreenSaver";

fn command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program).args(args).output()
        .map_err(|e| format!("Cannot run {program}: {e}"))?;
    if !output.status.success() {
        return Err(format!("{program} failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn get(key: &str) -> Result<String, String> {
    command("gsettings", &["get", SCHEMA, key])
}
fn set(key: &str, value: &str) -> Result<(), String> {
    command("gsettings", &["set", SCHEMA, key, value]).map(|_| ())
}
fn active() -> Result<bool, String> {
    let response = command("gdbus", &["call", "--session", "--dest", DEST,
        "--object-path", OBJECT, "--method", "org.mate.ScreenSaver.GetActive"])?;
    match response.as_str() {
        "(true,)" => Ok(true),
        "(false,)" => Ok(false),
        _ => Err(format!("Unexpected MATE GetActive response: {response}")),
    }
}
fn config_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        let path = PathBuf::from(path);
        if path.is_absolute() { return Ok(path.join("screenshaver")); }
    }
    Ok(PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unset")?)
        .join(".config/screenshaver"))
}
fn snapshot_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("mate-lock-test.restore"))
}
fn marker_path() -> Result<PathBuf, String> {
    let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or("XDG_RUNTIME_DIR is unset")?);
    if !runtime.is_absolute() { return Err("XDG_RUNTIME_DIR must be absolute".into()); }
    Ok(runtime.join("screenshaver-mate-test.owner"))
}
fn process_start(pid: u32) -> Result<String, String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|e| format!("Cannot inspect process {pid}: {e}"))?;
    let rest = stat.rsplit_once(") ").ok_or("Malformed /proc stat")?.1;
    // The first field after the process name is field 3; starttime is field 22.
    Ok(rest.split_whitespace().nth(19).ok_or("Missing process start time")?.to_owned())
}
fn owner_identity() -> Result<String, String> {
    let pid = std::process::id();
    Ok(format!("{pid} {}", process_start(pid)?))
}
pub(crate) fn presenter_authorized() -> Result<bool, String> {
    let value = match fs::read_to_string(marker_path()?) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    let mut parts = value.split_whitespace();
    let pid = parts.next().and_then(|s| s.parse::<u32>().ok());
    let start = parts.next();
    if parts.next().is_some() { return Ok(false); }
    match (pid, start) {
        (Some(pid), Some(start)) => Ok(process_start(pid).map(|v| v == start).unwrap_or(false)),
        _ => Ok(false),
    }
}
fn restore(snapshot: &std::path::Path) -> Result<(), String> {
    let data = fs::read_to_string(snapshot).map_err(|e| e.to_string())?;
    let mut lines = data.lines();
    if lines.next() != Some("mate-test-v1") { return Err("Invalid recovery snapshot version".into()); }
    let mode = lines.next().ok_or("Snapshot missing mode")?;
    let themes = lines.next().ok_or("Snapshot missing themes")?;
    if lines.next().is_some() { return Err("Malformed recovery snapshot".into()); }
    // Restore theme before mode to avoid selecting an invalid native mode/theme combination.
    set("themes", themes)?;
    set("mode", mode)?;
    fs::remove_file(snapshot).map_err(|e| e.to_string())?;
    println!("[MATE TEST] Previous native MATE settings restored.");
    Ok(())
}
fn theme_path() -> Result<PathBuf, String> {
    let data = match std::env::var_os("XDG_DATA_HOME") {
        Some(path) if PathBuf::from(&path).is_absolute() => PathBuf::from(path),
        _ => PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unset")?)
            .join(".local/share"),
    };
    Ok(data.join("applications/screensavers/screenshaver-mate-test.desktop"))
}

/// Append-only diagnostics shared by the test owner and MATE-launched child.
/// This file is deliberately separate from the normal runtime log.
pub(crate) fn diagnostic(message: &str) {
    use std::io::Write;
    let Ok(directory) = config_dir() else { return; };
    if fs::create_dir_all(&directory).is_err() { return; }
    let path = directory.join("mate-lock-presentation.log");
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) else { return; };
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
    }
    let _ = writeln!(file, "[MATE PROBE pid={}] {}", std::process::id(), message);
}

pub(crate) fn run() -> Result<(), String> {
    if !crate::detect_desktop_environment::detect().is_mate()
        || std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("x11") {
        return Err("--test-mate-lock requires a MATE/X11 session".into());
    }
    diagnostic("Test owner entered --test-mate-lock");
    let snapshot = snapshot_path()?;
    if snapshot.exists() {
        if active()? { return Err("MATE is locked; unlock before recovering an interrupted test".into()); }
        println!("[MATE TEST] Recovering previous interrupted test.");
        restore(&snapshot)?;
    }
    if active()? { return Err("Unlock MATE before running this test".into()); }
    let theme = theme_path()?;
    // The launcher is installed separately in MATE's approved engine directory.
    // Do not attempt privileged installation or modify system files here.
    const LAUNCHER: &str = "/usr/libexec/mate-screensaver/screenshaver-mate-probe";
    if !std::path::Path::new(LAUNCHER).is_file() {
        return Err(format!("MATE-approved launcher missing: {LAUNCHER}"));
    }
    let entry = format!("[Desktop Entry]\nName=Screenshaver MATE Launch Probe\nExec={LAUNCHER}\nTryExec={LAUNCHER}\nStartupNotify=false\nTerminal=false\nType=Application\nCategories=Screensaver;\nOnlyShowIn=MATE;\n");
    if !theme.exists() {
        fs::create_dir_all(theme.parent().ok_or("Invalid theme path")?)
            .map_err(|e| e.to_string())?;
        // create_new ensures we cannot overwrite a foreign file.
        use std::io::Write;
        let mut file = fs::OpenOptions::new().write(true).create_new(true)
            .open(&theme).map_err(|e| e.to_string())?;
        file.write_all(entry.as_bytes()).map_err(|e| e.to_string())?;
        diagnostic(&format!("Prepared MATE presenter theme={}", theme.display()));
        println!("[MATE TEST] Test theme registered. Open mate-screensaver-preferences and verify 'Screenshaver MATE Launch Probe' is listed.");
        println!("[MATE TEST] Close preferences, then run screenshaver --test-mate-lock again to perform the lock test.");
        return Ok(());
    }
    if fs::read_to_string(&theme).map_err(|e| e.to_string())? != entry {
        return Err(format!("Existing theme differs from this test's expected contents; refusing to replace: {}", theme.display()));
    }
    diagnostic("Previously registered test theme verified; proceeding with lock test");
    let marker = marker_path()?;
    if marker.exists() {
        if presenter_authorized()? { return Err("Another MATE test owner is active".into()); }
        fs::remove_file(&marker).map_err(|e| e.to_string())?;
    }
    let mode = get("mode")?;
    let themes = get("themes")?;
    if mode.contains('\n') || themes.contains('\n') { return Err("Unexpected multiline MATE settings".into()); }
    fs::create_dir_all(snapshot.parent().ok_or("Invalid snapshot path")?)
        .map_err(|e| e.to_string())?;
    fs::write(&snapshot, format!("mate-test-v1\n{mode}\n{themes}\n"))
        .map_err(|e| e.to_string())?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&snapshot, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    let result = (|| -> Result<(), String> {
        diagnostic(&format!("Using pre-registered presenter theme={}", theme.display()));
        fs::write(&marker, owner_identity()?).map_err(|e| e.to_string())?;
        set("themes", &format!("['{THEME}']"))?;
        set("mode", "single")?;
        diagnostic(&format!("Selected theme: mode={} themes={}", get("mode")?, get("themes")?));
        println!("[MATE TEST] Requesting native secure lock. Unlock normally to finish.");
        // MATE may not reply to Lock until the screen is unlocked. Never
        // synchronously wait for the method call before polling GetActive.
        let mut lock_request = Command::new("gdbus")
            .args(["call", "--session", "--dest", DEST,
                "--object-path", OBJECT, "--method", "org.mate.ScreenSaver.Lock"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Cannot start native MATE lock request: {e}"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut activated = false;
        while Instant::now() < deadline {
            if active()? { activated = true; break; }
            thread::sleep(Duration::from_millis(100));
        }
        if !activated {
            let _ = lock_request.kill();
            let _ = lock_request.wait();
            return Err("MATE did not report active within five seconds".into());
        }
        println!("[MATE TEST] Native MATE lock active. Check shader and authentication display.");
        diagnostic("Native MATE GetActive=true");
        // Never restore the theme while the session is still locked.
        while active()? { thread::sleep(Duration::from_millis(250)); }
        println!("[MATE TEST] MATE unlocked.");
        diagnostic("Native MATE GetActive=false");
        if lock_request.try_wait().map_err(|e| e.to_string())?.is_none() {
            let _ = lock_request.kill();
            let _ = lock_request.wait();
        }
        Ok(())
    })();
    let restored = if active().unwrap_or(true) {
        eprintln!("[MATE TEST] Still locked or unable to query MATE; recovery snapshot retained.");
        false
    } else {
        match restore(&snapshot) {
            Ok(()) => true,
            Err(e) => { eprintln!("[MATE TEST] Restoration failed: {e}"); false }
        }
    };
    if restored {
        let _ = fs::remove_file(&marker);
        // Keep the registered theme for subsequent tests; no system launcher is removed.
    }
    println!("[MATE TEST] Presenter diagnostics: {}", config_dir()?.join("mate-lock-presentation.log").display());
    diagnostic(&format!("Test completed: result={:?}, settings_restored={restored}", result));
    result
}
