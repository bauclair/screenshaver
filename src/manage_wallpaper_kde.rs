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
// The appletsrc file is asynchronous evidence, not a live Plasma acknowledgment.
// In particular, an old value observed immediately after a selection request
// cannot prove that a newer request will not overwrite it later.
fn wait_for_settled(assignments: &[Assignment]) -> Result<(), String> {
    let started = Instant::now();
    let deadline = started + Duration::from_secs(8);
    let mut matching_since: Option<Instant> = None;
    let mut previous_observation: Option<String> = None;
    let expected = assignments.iter()
        .map(|a| format!("{}={}", a.id, a.plugin))
        .collect::<Vec<_>>().join(", ");
    eprintln!("[KDE WALLPAPER SETTLE] begin; expected=[{expected}]; stable_required_ms=2000; timeout_ms=8000");
    loop {
        let (matches, observation) = match inventory() {
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
                (matches, details)
            }
            Err(error) => (false, format!("inventory_error={error}")),
        };
        // Log on transitions, not every 100 ms: avoids flooding runtime logs.
        if previous_observation.as_deref() != Some(observation.as_str()) {
            eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; observation=[{}]",
                started.elapsed().as_millis(), observation);
            previous_observation = Some(observation);
        }
        if matches {
            if matching_since.is_none() {
                eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; stability window started",
                    started.elapsed().as_millis());
            }
            let since = matching_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(2) {
                eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; persisted assignment stable for 2000 ms (not live presentation proof)",
                    started.elapsed().as_millis());
                return Ok(());
            }
        } else if matching_since.take().is_some() {
            eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; stability window reset",
                started.elapsed().as_millis());
        }
        if Instant::now() >= deadline {
            eprintln!("[KDE WALLPAPER SETTLE] elapsed_ms={}; timeout; last_observation=[{}]",
                started.elapsed().as_millis(), previous_observation.as_deref().unwrap_or("<none>"));
            return Err("Plasma persisted wallpaper did not remain stable for two seconds".into());
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
    let current = inventory()?;
    for item in &saved {
        let actual = current.iter().find(|c| c.id == item.id)
            .ok_or("Desktop containment disappeared; journal retained")?;
        if actual.plugin != TARGET && actual.plugin != item.plugin {
            return Err(format!("Containment {} changed to {}; journal retained for manual recovery",
                item.id, actual.plugin));
        }
    }
    select(&saved).map_err(|e| format!("Explicit restoration request failed: {e}; journal retained"))?;
    wait_for_settled(&saved).map_err(|e| format!("Restoration not settled: {e}; journal retained"))?;
    if retain_journal {
        return Ok(());
    }
    fs::remove_file(journal()?).map_err(|e| format!("Wallpaper appears restored, but journal removal failed: {e}"))
}

pub struct KdeWallpaperIntegrationGuard { active: bool }
impl KdeWallpaperIntegrationGuard {
    pub fn activate() -> Result<Self,String> {
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
            eprintln!("[KDE WALLPAPER] Previous restoration journal resolved after explicit Plasma restoration and settled configuration; native readiness remains independently verified");
        }
        let original = inventory()?;
        if original.iter().any(|a| a.plugin == TARGET) {
            return Err("Screenshaver is already selected as a wallpaper; original selection is unknown".into());
        }
        let package = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty())
            .map(PathBuf::from).or_else(|| env::var("HOME").ok().map(|s| PathBuf::from(s).join(".local/share")))
            .ok_or("Cannot resolve KDE package directory")?
            .join("plasma/wallpapers").join(TARGET).join("metadata.json");
        if !package.is_file() { return Err(format!("KDE wallpaper package not installed: {}",package.display())); }
        write_journal(&original)?;
        let desired: Vec<_> = original.iter().map(|a| Assignment { id:a.id, plugin:TARGET.into() }).collect();
        if let Err(activation_error) = select(&desired).and_then(|_|wait_for_native_presentation()) {
            // A failed activation can already have changed the live Plasma
            // wallpaper. No guard exists yet, so rollback must happen here.
            // Keep the durable journal unless restoration is verified.
            // A live presentation may have been created before the failure;
            // explicitly restore the prior wallpaper selection.
            eprintln!("[KDE WALLPAPER] Activation failed: {activation_error}; attempting immediate rollback");
            return match restore(true) {
                Ok(()) => Err(format!(
                    "Activation unverified: {activation_error}; restoration requested and persisted value settled, but live state unverified; journal retained"
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
        restore(false)?;
        self.active = false;
        Ok(())
    }
}
impl Drop for KdeWallpaperIntegrationGuard {
    fn drop(&mut self) { if self.active { if let Err(e) = self.deactivate() { eprintln!("[KDE WALLPAPER] Restoration pending: {e}"); } } }
}
