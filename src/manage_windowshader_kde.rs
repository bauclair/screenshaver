//! Process-scoped KDE/KWin Windowshader integration. Non-KDE desktops are no-ops.
use std::path::PathBuf;
use std::process::Command;

const SCRIPT: &str = r###"// Temporary, process-owned KWin script for Screenshaver Windowshader.
(function () {
    var prefix = '[SCREENSHAVER-KWIN] ';
    function matches(w) {
        return w && String(w.caption || '') === 'Screenshaver Windowshader'
            && (String(w.resourceClass || '').toLowerCase() === 'screenshaver'
                || String(w.resourceName || '').toLowerCase() === 'screenshaver');
    }
    function attach(w) {
        if (!matches(w)) return;
        try {
            w.keepAbove = true;
            print(prefix + 'Windowshader keepAbove=' + w.keepAbove);
        } catch (e) { print(prefix + 'keepAbove error: ' + e); }
    }
    workspace.windowList().forEach(attach);
    workspace.windowAdded.connect(attach);
})();
"###;

pub(crate) struct WindowshaderKwinGuard {
    qdbus: &'static str,
    object_path: String,
    script_path: PathBuf,
}

impl WindowshaderKwinGuard {
    pub(crate) fn start() -> Result<Option<Self>, String> {
        if !crate::detect_desktop_environment::detect().is_kde_plasma()
            || std::env::var("XDG_SESSION_TYPE").ok().as_deref() != Some("wayland") {
            return Ok(None);
        }
        let qdbus = ["qdbus6", "qdbus"].into_iter().find(|binary| {
            Command::new(binary).arg("--help").output().is_ok()
        }).ok_or("Neither qdbus6 nor qdbus is installed")?;
        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from).ok_or("XDG_RUNTIME_DIR is unavailable")?;
        let script_path = dir.join(format!("screenshaver-windowshader-{}.js", std::process::id()));
        std::fs::write(&script_path, SCRIPT).map_err(|e| e.to_string())?;
        let name = format!("screenshaver-windowshader-{}", std::process::id());
        let load = Command::new(qdbus).args([
            "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.loadScript",
        ]).arg(&script_path).arg(&name).output().map_err(|e| e.to_string())?;
        if !load.status.success() {
            let _ = std::fs::remove_file(&script_path);
            return Err(format!("KWin loadScript failed: {}", String::from_utf8_lossy(&load.stderr)));
        }
        let id = String::from_utf8_lossy(&load.stdout).trim().to_string();
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
            let _ = std::fs::remove_file(&script_path);
            return Err(format!("Unexpected KWin script identifier: {id}"));
        }
        let object_path = format!("/Scripting/Script{id}");
        let guard = Self { qdbus, object_path, script_path };
        let run = Command::new(qdbus).args([
            "org.kde.KWin", &guard.object_path, "org.kde.kwin.Script.run",
        ]).output().map_err(|e| e.to_string())?;
        if !run.status.success() {
            return Err(format!("KWin script run failed: {}", String::from_utf8_lossy(&run.stderr)));
        }
        Ok(Some(guard))
    }
}

impl Drop for WindowshaderKwinGuard {
    fn drop(&mut self) {
        let _ = Command::new(self.qdbus).args([
            "org.kde.KWin", &self.object_path, "org.kde.kwin.Script.stop",
        ]).output();
        let _ = std::fs::remove_file(&self.script_path);
    }
}
