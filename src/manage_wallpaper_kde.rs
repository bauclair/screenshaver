//! KDE Plasma wallpaper selection lifecycle. Never edits appletsrc directly.
//! This guard controls Plasma selection, NOT native/legacy rendering ownership.
use std::{env,fs,io::{Read,Write},path::PathBuf,process::{Command,Stdio},thread,time::{Duration,Instant}};

const TARGET: &str = "org.screenshaver.wallpaper";
const MAGIC: &str = "screenshaver-kde-wallpaper-v1";

#[derive(Clone,Debug)]
struct Assignment { id: u64, plugin: String }

fn appletsrc() -> Result<PathBuf,String> {
    let home = env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty())
        .map(PathBuf::from).or_else(|| env::var("HOME").ok().map(|s| PathBuf::from(s).join(".config")))
        .ok_or("Cannot resolve KDE configuration directory")?;
    Ok(home.join("plasma-org.kde.plasma.desktop-appletsrc"))
}
fn journal() -> Result<PathBuf,String> {
    let base = env::var("XDG_STATE_HOME").ok().filter(|s| !s.is_empty())
        .map(PathBuf::from).or_else(|| env::var("HOME").ok().map(|s| PathBuf::from(s).join(".local/state")))
        .ok_or("Cannot resolve state directory")?;
    Ok(base.join("screenshaver/kde-wallpaper-restore-v1"))
}
/// Locate the installed wallpaper package using the XDG data search path.
/// KDE accepts both user-local and system-wide wallpaper packages.
fn locate_wallpaper_package() -> Result<PathBuf, String> {
    let mut data_dirs = Vec::<PathBuf>::new();
    if let Some(home) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        data_dirs.push(PathBuf::from(home));
    } else if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        data_dirs.push(PathBuf::from(home).join(".local/share"));
    }

    // An unset or empty XDG_DATA_DIRS uses the freedesktop defaults.
    let system_dirs = env::var_os("XDG_DATA_DIRS").filter(|value| !value.is_empty());
    if let Some(value) = system_dirs {
        data_dirs.extend(env::split_paths(&value).filter(|path| !path.as_os_str().is_empty()));
    } else {
        data_dirs.push(PathBuf::from("/usr/local/share"));
        data_dirs.push(PathBuf::from("/usr/share"));
    }

    // Preserve support for NixOS profiles and conventional system locations
    // even when a session provides an incomplete XDG_DATA_DIRS value.
    for fallback in ["/run/current-system/sw/share", "/usr/local/share", "/usr/share"] {
        let path = PathBuf::from(fallback);
        if !data_dirs.contains(&path) {
            data_dirs.push(path);
        }
    }

    let mut searched = Vec::new();
    for dir in data_dirs {
        let candidate = dir.join("plasma/wallpapers").join(TARGET).join("metadata.json");
        if candidate.is_file() {
            return Ok(candidate);
        }
        searched.push(candidate.display().to_string());
    }
    Err(format!("KDE wallpaper package not installed; checked: {}", searched.join(", ")))
}

fn inventory() -> Result<Vec<Assignment>,String> {
    let data = fs::read_to_string(appletsrc()?).map_err(|e| format!("Cannot inspect Plasma desktop configuration: {e}"))?;
    let mut id = None;
    let mut desktop = false;
    let mut plugin = None;
    let mut wallpaper = None;
    let mut result = Vec::new();
    let mut flush = |id: Option<u64>, desktop: bool, plugin: &Option<String>, wallpaper: &Option<String>| {
        if desktop && plugin.as_deref() == Some("org.kde.plasma.folder") {
            if let (Some(id),Some(wallpaper)) = (id,wallpaper) {
                result.push(Assignment { id, plugin: wallpaper.clone() });
            }
        }
    };
    for line in data.lines().chain(std::iter::once("[END]")) {
        if line.starts_with('[') {
            flush(id, desktop, &plugin, &wallpaper);
            id = None; desktop = false; plugin = None; wallpaper = None;
            if let Some(raw) = line.strip_prefix("[Containments][").and_then(|s| s.strip_suffix(']')) {
                if let Ok(number) = raw.parse::<u64>() { id = Some(number); desktop = true; }
            }
        } else if desktop {
            if let Some(value) = line.strip_prefix("plugin=") { plugin = Some(value.to_owned()); }
            if let Some(value) = line.strip_prefix("wallpaperplugin=") { wallpaper = Some(value.to_owned()); }
        }
    }
    if result.is_empty() { return Err("No KDE folder desktop wallpaper containments identified".into()); }
    Ok(result)
}
fn execute(script: &str) -> Result<(),String> {
    // Plasma evaluateScript normally returns no useful output. Verify state separately.
    let command = if Command::new("qdbus6").arg("--help").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok() { "qdbus6" } else { "qdbus" };
    let mut child = Command::new(command)
        .args(["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.evaluateScript", script])
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Cannot invoke Plasma script interface: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() { let _ = pipe.read_to_string(&mut stderr); }
                if status.success() { return Ok(()); }
                return Err(format!("Plasma script call failed ({status}): {stderr}"));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => { let _ = child.kill(); let _ = child.wait(); return Err("Plasma script timed out".into()); }
            Err(e) => return Err(format!("Cannot wait for Plasma script: {e}")),
        }
    }
}
fn select(assignments: &[Assignment]) -> Result<(),String> {
    if assignments.is_empty() { return Ok(()); }
    let mut script = String::from("(function(){var ds=desktops();");
    for a in assignments {
        // Numeric containment IDs and validated plugin IDs only; no untrusted JS.
        if !a.plugin.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') {
            return Err("Invalid wallpaper plugin ID".into());
        }
        script.push_str(&format!("var found=false;for(var i=0;i<ds.length;i++){{if(Number(ds[i].id)==={}){{ds[i].wallpaperPlugin='{}';found=true;break;}}}}if(!found)throw Error('containment {} missing');", a.id,a.plugin,a.id));
    }
    script.push_str("})();");
    execute(&script)
}
// Plasma may instantiate a wallpaper item without immediately persisting its
// assignment. Observe the live native monitor, not appletsrc, for activation.
// This is presentation evidence only; the coordinator alone grants ownership.
fn wait_for_native_presentation() -> Result<(), String> {
    use crate::control_wallpaper::{NativePresentationAssessment, NativePresentationPreflight};
    use crate::manage_wallpaper_ownership::PlasmaInstance;

    let deadline = Instant::now() + Duration::from_secs(12);
    let mut last_reason = String::from("No native wallpaper presentation observed");
    loop {
        match crate::control_wallpaper::assess_native_presentations() {
            NativePresentationAssessment::Observed(items) if !items.is_empty() => {
                let first = &items[0];
                if items.iter().all(|item| item.pid == first.pid && item.start_ticks == first.start_ticks) {
                    let expected = PlasmaInstance { pid: first.pid, start_ticks: first.start_ticks };
                    match crate::control_wallpaper::assess_native_handoff_preflight(&expected) {
                        NativePresentationPreflight::Candidate { associated_nodes, .. }
                            if associated_nodes > 0 => return Ok(()),
                        NativePresentationPreflight::NotReady(reason)
                        | NativePresentationPreflight::Unverified(reason) => last_reason = reason,
                        other => last_reason = format!("Native monitor preflight: {other:?}"),
                    }
                } else {
                    last_reason = "Native wallpaper presentations belong to conflicting Plasma instances".into();
                }
            }
            NativePresentationAssessment::Absent => {
                last_reason = "No native wallpaper presentation observed".into();
            }
            NativePresentationAssessment::Unverified(error) => last_reason = error,
            _ => {}
        }
        if Instant::now() >= deadline {
            return Err(format!("Live Plasma wallpaper activation not confirmed: {last_reason}"));
        }
        thread::sleep(Duration::from_millis(100));
    }
}
fn write_journal(items: &[Assignment]) -> Result<(),String> {
    let path = journal()?;
    if path.exists() { return Err(format!("Previous KDE wallpaper restoration journal exists: {}",path.display())); }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
    let mut text = format!("{MAGIC}\n");
    for item in items { text.push_str(&format!("{}\t{}\n",item.id,item.plugin)); }
    file.write_all(text.as_bytes()).and_then(|_|file.sync_all()).map_err(|e| e.to_string())
}
fn read_journal() -> Result<Option<Vec<Assignment>>,String> {
    let path = journal()?;
    let text = match fs::read_to_string(&path) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut lines = text.lines();
    if lines.next() != Some(MAGIC) { return Err("Unknown KDE wallpaper journal format; refusing restoration".into()); }
    let mut out = Vec::new();
    for line in lines {
        let (id,plugin) = line.split_once('\t').ok_or("Malformed wallpaper journal")?;
        let id = id.parse::<u64>().map_err(|e| e.to_string())?;
        if plugin.is_empty() { return Err("Empty saved wallpaper plugin".into()); }
        out.push(Assignment { id, plugin: plugin.to_owned() });
    }
    if out.is_empty() { return Err("Empty wallpaper restoration journal".into()); }
    Ok(Some(out))
}
// Query the actual Plasma desktop objects, rather than the asynchronously
// persisted appletsrc file. This is configuration selection evidence, NOT proof
// of compositor scene retirement or native ownership revocation.
fn live_inventory() -> Result<Vec<Assignment>, String> {
    // print() is required: evaluateScript does not return the last expression.
    let script = "print(desktops().map(function(d){return String(d.id)+'='+String(d.wallpaperPlugin);}).join('\\n'))";
    let command = if Command::new("qdbus6").arg("--help")
        .stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok() {
        "qdbus6"
    } else { "qdbus" };
    let mut child = Command::new(command)
        .args(["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.evaluateScript", script])
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Cannot query live Plasma desktops: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill(); let _ = child.wait();
                return Err("Live Plasma desktop query timed out".into());
            }
            Err(e) => return Err(format!("Cannot wait for live Plasma query: {e}")),
        }
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout).map_err(|e| e.to_string())?;
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    if !status.success() {
        return Err(format!("Live Plasma query failed ({status}): {stderr}"));
    }
    let mut result = Vec::new();
    for line in stdout.lines().map(str::trim).filter(|s| !s.is_empty()) {
        let (id, plugin) = line.split_once('=')
            .ok_or_else(|| format!("Unexpected live Plasma assignment: {line:?}"))?;
        let id = id.parse::<u64>()
            .map_err(|_| format!("Invalid live Plasma desktop ID: {id:?}"))?;
        if plugin.is_empty() || result.iter().any(|a: &Assignment| a.id == id) {
            return Err("Empty or duplicate live Plasma wallpaper assignment".into());
        }
        result.push(Assignment { id, plugin: plugin.to_owned() });
    }
    if result.is_empty() {
        return Err(format!("Live Plasma query returned no assignments: {stdout:?}"));
    }
    Ok(result)
}

// A stable live selection is the primary restoration criterion. The persisted
// appletsrc value is reported as supplemental evidence and is not allowed to
// stall a verified live restoration.
fn wait_for_settled(assignments: &[Assignment], _retry_restoration: bool) -> Result<(), String> {
    let started = Instant::now();
    let deadline = started + Duration::from_secs(8);
    let mut matching_since: Option<Instant> = None;
    let mut previous_observation: Option<String> = None;
    let expected = assignments.iter().map(|a| format!("{}={}", a.id, a.plugin))
        .collect::<Vec<_>>().join(", ");
    eprintln!("[KDE WALLPAPER SETTLE] begin; source=live_plasma; expected=[{expected}]; stable_required_ms=2000; timeout_ms=8000");
    loop {
        let (matches, observation, conflict) = match live_inventory() {
            Ok(current) => {
                let details = assignments.iter().map(|a| {
                    match current.iter().find(|c| c.id == a.id) {
                        Some(c) => format!("{}: expected={} actual={} match={}",
                            a.id, a.plugin, c.plugin, c.plugin == a.plugin),
                        None => format!("{}: expected={} actual=<missing> match=false", a.id, a.plugin),
                    }
                }).collect::<Vec<_>>().join("; ");
                let matches = assignments.iter().all(|a| current.iter()
                    .any(|c| c.id == a.id && c.plugin == a.plugin));
                let conflict = assignments.iter().any(|a| current.iter()
                    .find(|c| c.id == a.id)
                    .is_some_and(|c| c.plugin != a.plugin && c.plugin != TARGET));
                (matches, details, conflict)
            }
            Err(error) => (false, format!("live_query_error={error}"), false),
        };
        if previous_observation.as_deref() != Some(observation.as_str()) {
            eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; observation=[{}]",
                started.elapsed().as_millis(), observation);
            previous_observation = Some(observation);
        }
        if conflict {
            return Err("Live Plasma wallpaper changed to a third-party plugin; journal retained".into());
        }
        if matches {
            if matching_since.is_none() {
                eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; live stability window started",
                    started.elapsed().as_millis());
            }
            let since = matching_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(2) {
                match inventory() {
                    Ok(persisted) => {
                        let persisted_matches = assignments.iter().all(|a| persisted.iter()
                            .any(|c| c.id == a.id && c.plugin == a.plugin));
                        eprintln!("[KDE WALLPAPER SETTLE] live assignment stable for 2000 ms; persisted_match={} (supplemental, not live presentation proof)", persisted_matches);
                    }
                    Err(error) => eprintln!("[KDE WALLPAPER SETTLE] live assignment stable for 2000 ms; persisted_query_error={error}"),
                }
                return Ok(());
            }
        } else if matching_since.take().is_some() {
            eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; live stability window reset",
                started.elapsed().as_millis());
        }
        if Instant::now() >= deadline {
            eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; timeout; last_observation=[{}]",
                started.elapsed().as_millis(), previous_observation.as_deref().unwrap_or("<none>"));
            return Err("Live Plasma wallpaper assignment did not remain stable for two seconds".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

// Never infer that rollback is unnecessary from a stale appletsrc snapshot.
// Always issue an explicit restoration request for every saved containment.
// The journal is kept on an activation failure even if the persisted value
// settles, because no authoritative live-state verification is available.
fn restore(retain_journal: bool) -> Result<(),String> {
    let Some(saved) = read_journal()? else { return Ok(()); };
    // The persisted appletsrc can lag the running Plasma process. Check the
    // live desktop assignments before deciding whether rollback is safe.
    let current = live_inventory().map_err(|e| format!("Cannot verify live Plasma state before restoration: {e}; journal retained"))?;
    for item in &saved {
        let actual = current.iter().find(|c| c.id == item.id)
            .ok_or("Desktop containment disappeared; journal retained")?;
        if actual.plugin != TARGET && actual.plugin != item.plugin {
            return Err(format!("Containment {} changed to {}; journal retained for manual recovery",
                item.id, actual.plugin));
        }
    }
    select(&saved).map_err(|e| format!("Explicit restoration request failed: {e}; journal retained"))?;
    wait_for_settled(&saved, true).map_err(|e| format!("Restoration not settled: {e}; journal retained"))?;
    if retain_journal {
        return Ok(());
    }
    fs::remove_file(journal()?).map_err(|e| format!("Live Plasma wallpaper selection restored, but journal removal failed: {e}"))
}

pub struct KdeWallpaperIntegrationGuard { active: bool }
impl KdeWallpaperIntegrationGuard {
    pub fn activate() -> Result<Self,String> {
        Self::activate_with_readiness(wait_for_native_presentation)
    }

    /// Use the new frame-presenter readiness protocol instead of the legacy
    /// NativeOpenGLUnderlay monitor. Caller must start producer first.
    pub fn activate_with_readiness<F>(mut readiness: F) -> Result<Self,String>
    where F: FnMut() -> Result<(),String> {

        // Resolve a previous interrupted transaction before capturing a new
        // baseline. Never delete the journal merely because an appletsrc
        // snapshot happens to match: request restoration explicitly and wait
        // for the persisted selection to settle. A failure retains the journal.
        // This is configuration-level recovery, not proof of live QML rendering;
        // the native ownership coordinator still requires its own live evidence.
        if read_journal()?.is_some() {
            restore(false).map_err(|error| format!(
                "Previous KDE wallpaper transaction could not be restored; journal retained: {error}"
            ))?;
            eprintln!("[KDE WALLPAPER] Previous restoration journal resolved after explicit Plasma restoration and stable live selection; native readiness remains independently verified");
        }
        // Capture the live assignment that will actually be replaced. A stale
        // appletsrc value must not become the recovery baseline.
        let original = live_inventory()?;
        if original.iter().any(|a| a.plugin == TARGET) {
            return Err("Screenshaver is already selected as a wallpaper; original selection is unknown".into());
        }
        let package = locate_wallpaper_package()?;
        // Record the live selection durably before requesting any change.
        // Plasma retains wallpaper-plugin-specific configuration when its
        // wallpaperPlugin selection changes; we do not rewrite those groups.
        write_journal(&original)?;
        let desired: Vec<_> = original.iter().map(|a| Assignment { id:a.id, plugin:TARGET.into() }).collect();
        if let Err(activation_error) = select(&desired).and_then(|_|readiness()) {
            // A failed activation can already have changed the live Plasma
            // wallpaper. No guard exists yet, so rollback must happen here.
            // Keep the durable journal unless restoration is verified.
            // A live presentation may have been created before the failure;
            // explicitly restore the prior wallpaper selection.
            eprintln!("[KDE WALLPAPER] Activation failed: {activation_error}; attempting immediate rollback");
            return match restore(true) {
                Ok(()) => Err(format!(
                    "Activation unverified: {activation_error}; restoration requested and live selection settled, but native scene retirement unverified; journal retained"
                )),
                Err(rollback_error) => Err(format!(
                    "Activation unverified: {activation_error}; immediate rollback unverified: {rollback_error}; restoration journal retained"
                )),
            };
        }
        Ok(Self { active: true })
    }
    pub fn deactivate(&mut self) -> Result<(),String> {
        if !self.active { return Ok(()); }
        // This guard gets one shutdown restoration attempt. Mark it consumed
        // before attempting the fallible operation so Drop cannot initiate a
        // second eight-second wait after an explicit shutdown failure.
        // restore() preserves the durable recovery journal on failure.
        self.active = false;
        restore(false)
    }
}
impl Drop for KdeWallpaperIntegrationGuard {
    fn drop(&mut self) { if self.active { if let Err(e) = self.deactivate() { eprintln!("[KDE WALLPAPER] Restoration pending: {e}"); } } }
}
