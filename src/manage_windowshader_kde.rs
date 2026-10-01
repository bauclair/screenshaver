//! Process-scoped KDE/KWin Windowshader integration. Non-KDE desktops are no-ops.
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct PositionReceiver(Arc<Mutex<Option<(i32, i32)>>>);

#[zbus::interface(name = "org.screenshaver.Windowshader.Position")]
impl PositionReceiver {
    fn report(&self, x: i32, y: i32) {
        if let Ok(mut position) = self.0.lock() {
            *position = Some((x, y));
        }
    }
}


const SCRIPT: &str = r###"// Temporary, process-owned KWin script for Screenshaver Windowshader.
(function () {
    var prefix = '[SCREENSHAVER-KWIN] ';
    var restored = false;
    function matches(w) {
        return w && String(w.caption || '') === 'Screenshaver Windowshader'
            && (String(w.resourceClass || '').toLowerCase() === 'screenshaver'
                || String(w.resourceName || '').toLowerCase() === 'screenshaver');
    }
    function report(w) {
        if (!matches(w) || w.maximizeMode !== 0 && w.maximizeMode !== undefined) return;
        var g = w.frameGeometry;
        if (!g || !Number.isFinite(g.x) || !Number.isFinite(g.y)
            || g.width < 64 || g.height < 64) return;
        callDBus('__SERVICE__', '/Windowshader',
            'org.screenshaver.Windowshader.Position', 'Report',
            Math.round(g.x), Math.round(g.y));
    }
    function attach(w) {
        if (!matches(w)) return;
        try {
            w.keepAbove = true;
            if (!restored && __RESTORE__) {
                restored = true;
                var g = w.frameGeometry;
                w.frameGeometry = { x: __X__, y: __Y__, width: g.width, height: g.height };
            }
            report(w);
            w.frameGeometryChanged.connect(function () { report(w); });
            print(prefix + 'Windowshader keepAbove=' + w.keepAbove);
        } catch (e) { print(prefix + 'integration error: ' + e); }
    }
    workspace.windowList().forEach(attach);
    workspace.windowAdded.connect(attach);
})();
"###;

pub(crate) struct WindowshaderKwinGuard {
    qdbus: &'static str,
    object_path: String,
    script_path: PathBuf,
    _connection: zbus::blocking::Connection,
    _runtime: tokio::runtime::Runtime,
    position: Arc<Mutex<Option<(i32, i32)>>>,
}

impl WindowshaderKwinGuard {
    pub(crate) fn last_position(&self) -> Option<(i32, i32)> {
        self.position.lock().ok().and_then(|position| *position)
    }

    pub(crate) fn start(saved_position: Option<(i32, i32)>) -> Result<Option<Self>, String> {
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
        let service = format!("org.screenshaver.Windowshader.P{}", std::process::id());
        let position = Arc::new(Mutex::new(None));
        // zbus is built with the Tokio executor. Establish its connection and
        // object server within a dedicated runtime, which remains alive while
        // this process-scoped KWin integration is installed.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| format!("Unable to initialize Windowshader D-Bus runtime: {e}"))?;
        let connection = {
            let _entered = runtime.enter();
            let connection = zbus::blocking::Connection::session()
                .map_err(|e| e.to_string())?;
            connection.request_name(service.as_str()).map_err(|e| e.to_string())?;
            connection.object_server().at("/Windowshader", PositionReceiver(position.clone()))
                .map_err(|e| e.to_string())?;
            connection
        };
        let (restore, x, y) = match saved_position {
            Some((x, y)) => ("true", x, y),
            None => ("false", 0, 0),
        };
        let script = SCRIPT.replace("__SERVICE__", &service)
            .replace("__RESTORE__", restore)
            .replace("__X__", &x.to_string())
            .replace("__Y__", &y.to_string());
        std::fs::write(&script_path, script).map_err(|e| e.to_string())?;
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
        let guard = Self { qdbus, object_path, script_path, _connection: connection, _runtime: runtime, position };
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
