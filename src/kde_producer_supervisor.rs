//! Process-owned KDE Plasma frame producer supervisor.
//! This module does not select Plasma wallpapers or start a legacy renderer.
use std::{env,fs,io::{Read,Write,BufRead,BufReader},os::unix::fs::MetadataExt,path::{Path,PathBuf},process::{Child,ChildStdin,Command,Stdio},thread,sync::mpsc::{self,Receiver},time::{Duration,Instant}};

const HEADER_SIZE: usize = 64;
const MAGIC: &[u8;8] = b"SSPLFR01";

#[derive(Debug,Clone)]
pub struct ProducerSettings {
    pub executable: PathBuf,
    pub library: PathBuf,
    pub frame_file: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, serde::Deserialize)]
struct PlasmaOutput {
    id: u32,
    connector: String,
    enabled: bool,
    connected: bool,
    physical_width: u32,
    physical_height: u32,
    logical_width: u32,
    logical_height: u32,
    x: i32,
    y: i32,
    scale: f64,
    refresh_hz: f64,
}

// The KDE-specific native library is deployed beside the renderer library.
// No helper process, shell command, polling loop, or mutex is involved.
fn discover_geometry(renderer_library: &Path) -> Result<(u32,u32), String> {
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_void};
    let native_path = renderer_library
        .parent()
        .ok_or("KDE renderer library has no parent directory")?
        .join("libScreenshaverKScreen.so");
    if !native_path.is_file() {
        return Err(format!("KScreen discovery library not installed: {}", native_path.display()));
    }
    let name = CString::new(native_path.as_os_str().as_encoded_bytes())
        .map_err(|_| "Invalid native library path")?;
    unsafe {
        let handle = libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        if handle.is_null() {
            let error = libc::dlerror();
            return Err(format!("Cannot load KScreen discovery library: {}",
                if error.is_null() { "unknown dynamic linker error".into() }
                else { CStr::from_ptr(error).to_string_lossy().into_owned() }));
        }
        struct Library(*mut c_void);
        impl Drop for Library {
            fn drop(&mut self) { unsafe { libc::dlclose(self.0); } }
        }
        let _library = Library(handle);
        let symbol = CString::new("screenshaver_kscreen_discover").unwrap();
        let function = libc::dlsym(handle, symbol.as_ptr());
        if function.is_null() {
            return Err("KScreen discovery entry point missing".into());
        }
        type Discover = unsafe extern "C" fn(*mut c_char, usize) -> i32;
        let discover: Discover = std::mem::transmute(function);
        let mut buffer = vec![0_u8; 65536];
        let status = discover(buffer.as_mut_ptr().cast(), buffer.len());
        if status != 0 {
            return Err(format!("libkscreen discovery failed (status {status})"));
        }
        let json = CStr::from_ptr(buffer.as_ptr().cast()).to_bytes();
        let outputs: Vec<PlasmaOutput> = serde_json::from_slice(json)
            .map_err(|e| format!("Invalid structured KScreen output: {e}"))?;
        let active: Vec<_> = outputs.iter().filter(|o| o.enabled && o.connected).collect();
        match active.as_slice() {
            [one] => {
                if one.physical_width == 0 || one.physical_height == 0 ||
                   one.logical_width == 0 || one.logical_height == 0 ||
                   !one.scale.is_finite() || one.scale <= 0.0 {
                    return Err(format!("KScreen output {} has incomplete display metadata", one.connector));
                }
                eprintln!("[KDE GEOMETRY] id={} {} physical={}x{} logical={}x{} at {},{} scale={} refresh={}Hz",
                    one.id, one.connector, one.physical_width, one.physical_height,
                    one.logical_width, one.logical_height, one.x, one.y, one.scale, one.refresh_hz);
                Ok((one.physical_width, one.physical_height))
            }
            [] => Err("libkscreen reports no enabled, connected KDE display".into()),
            _ => Err(format!("libkscreen reports {} active displays; per-output rendering is not implemented", active.len())),
        }
    }
}

impl ProducerSettings {
    /// Paths are explicit: do not guess build artifacts or silently run stale binaries.
    /// Width/height should match the target Plasma output; Qt scales frames if needed.
    pub fn from_environment() -> Result<Self,String> {
        let path = |name: &str| -> Result<PathBuf,String> {
            let value = env::var_os(name).ok_or_else(|| format!("{name} must be configured"))?;
            let path = PathBuf::from(value);
            if !path.is_absolute() { return Err(format!("{name} must be an absolute path")); }
            Ok(path)
        };
        let number = |name: &str, fallback: u32| -> Result<u32,String> {
            env::var(name).ok().map(|s| s.parse::<u32>().map_err(|e| format!("{name}: {e}"))).unwrap_or(Ok(fallback))
        };
        let runtime = path("XDG_RUNTIME_DIR")?;
        let meta = fs::metadata(&runtime).map_err(|e| format!("Runtime directory: {e}"))?;
        if !meta.is_dir() { return Err("XDG_RUNTIME_DIR is not a directory".into()); }
        // Resolve native components from the same immutable package as this executable.
        // Developer overrides remain supported, but are never required in production.
        let installed_bin = env::current_exe()
            .map_err(|e| format!("Cannot resolve Screenshaver executable: {e}"))?;
        let package_root = installed_bin.parent()
            .and_then(Path::parent)
            .ok_or("Screenshaver executable is not inside a package bin directory")?;
        let native_dir = package_root.join("lib/screenshaver/kde");
        let installed_path = |name: &str, default: PathBuf| -> Result<PathBuf,String> {
            match env::var_os(name) {
                Some(value) => {
                    let override_path = PathBuf::from(value);
                    if !override_path.is_absolute() {
                        return Err(format!("{name} must be an absolute path"));
                    }
                    Ok(override_path)
                }
                None => Ok(default),
            }
        };
        let library = installed_path(
            "SCREENSHAVER_KDE_RENDERER_LIBRARY",
            native_dir.join("libscreenshaver.so"),
        )?;
        let executable = installed_path(
            "SCREENSHAVER_KDE_PRODUCER",
            package_root.join("bin/screenshaver-plasma-producer"),
        )?;
        if !library.is_file() {
            return Err(format!("KDE renderer library not installed: {}", library.display()));
        }
        if !executable.is_file() {
            return Err(format!("KDE producer executable not installed: {}", executable.display()));
        }
        let width_override=env::var("SCREENSHAVER_KDE_FRAME_WIDTH").ok();
        let height_override=env::var("SCREENSHAVER_KDE_FRAME_HEIGHT").ok();
        if width_override.is_some() != height_override.is_some() {
            return Err("Set both SCREENSHAVER_KDE_FRAME_WIDTH and SCREENSHAVER_KDE_FRAME_HEIGHT, or neither".into());
        }
        let (width,height)=if width_override.is_some() {
            (number("SCREENSHAVER_KDE_FRAME_WIDTH",0)?,number("SCREENSHAVER_KDE_FRAME_HEIGHT",0)?)
        } else { discover_geometry(&library)? };
        let settings = Self {
            executable,
            library,
            frame_file: runtime.join("screenshaver-plasma.frames"),
            width,
            height,
            fps: number("SCREENSHAVER_KDE_FRAME_FPS", 30)?,
        };
        if !settings.executable.is_file() || !settings.library.is_file() {
            return Err("Producer executable or renderer library is missing".into());
        }
        if settings.width == 0 || settings.height == 0 || settings.width > 16384 || settings.height > 16384 || !(1..=240).contains(&settings.fps) {
            return Err("Invalid producer dimensions or FPS".into());
        }
        let bytes = u64::from(settings.width) * u64::from(settings.height) * 4;
        if bytes.checked_mul(2).and_then(|n| n.checked_add(64)).is_none_or(|n| n > 256*1024*1024) {
            return Err("Producer frame transport exceeds 256 MiB".into());
        }
        Ok(settings)
    }
}

pub struct ProducerSupervisor {
    child: Child,
    frame_file: PathBuf,
    owned_inode: Option<(u64,u64)>,
    control: Option<ChildStdin>,
    acknowledgments: Receiver<String>,
}

impl ProducerSupervisor {
    pub fn start(settings: &ProducerSettings) -> Result<Self,String> {
        // The producer uses create_new and must never overwrite an orphan file.
        // Never delete a stale frame file automatically: a foreign producer may own it.
        if fs::symlink_metadata(&settings.frame_file).is_ok() {
            return Err(format!("Frame path already exists; inspect owner before retrying: {}", settings.frame_file.display()));
        }
        let mut child = Command::new(&settings.executable)
            .arg(&settings.library).arg(&settings.frame_file)
            .arg(settings.width.to_string()).arg(settings.height.to_string())
            .arg(settings.fps.to_string())
            .stdin(Stdio::piped()).stderr(Stdio::piped()).spawn()
            .map_err(|e| format!("Cannot launch KDE frame producer: {e}"))?;
        let control = child.stdin.take().ok_or("Producer stdin pipe unavailable")?;
        let stderr = child.stderr.take().ok_or("Producer stderr pipe unavailable")?;
        let (tx, acknowledgments) = mpsc::channel();
        thread::Builder::new().name("plasma-producer-ack".into()).spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(line) => {
                        // Keep normal diagnostics visible while forwarding control ACKs.
                        eprintln!("{line}");
                        if line.starts_with("[PLASMA CONTROL] ") && tx.send(line).is_err() { break; }
                    }
                    Err(_) => break,
                }
            }
        }).map_err(|e| format!("Cannot monitor producer acknowledgments: {e}"))?;
        let mut supervisor = Self { child, frame_file: settings.frame_file.clone(), owned_inode: None, control: Some(control), acknowledgments };
        if let Err(error) = supervisor.wait_for_frames(Duration::from_secs(12)) {
            supervisor.stop();
            return Err(error);
        }
        Ok(supervisor)
    }

    /// Send a control request and wait for the render thread's state transition.
    /// The acknowledgment is emitted after the command is applied, not on receipt.
    /// This protocol is single-flight: callers must not issue concurrent requests.
    fn command(&mut self, request: &str, expected: &str, timeout: Duration) -> Result<u64,String> {
        if !self.is_alive()? { return Err("KDE producer exited before control request".into()); }
        let input = self.control.as_mut().ok_or("Producer command pipe is closed")?;
        writeln!(input, "{request}").and_then(|_| input.flush())
            .map_err(|e| format!("Cannot send {request} to KDE producer: {e}"))?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err(format!("Producer {request} acknowledgment timed out")); }
            match self.acknowledgments.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(line) => {
                    if let Some(sequence) = parse_control_ack(&line, expected) { return Ok(sequence); }
                    if line.starts_with("[PLASMA CONTROL] UNSUPPORTED") {
                        return Err(format!("Producer rejected command {request}: {line}"));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !self.is_alive()? { return Err(format!("Producer exited awaiting {request} acknowledgment")); }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Producer acknowledgment stream closed".into()),
            }
        }
    }

    pub fn pause(&mut self, timeout: Duration) -> Result<u64,String> {
        let sequence = self.command("PAUSE", "PAUSED", timeout)?;
        let (current, identity) = read_frame_header(&self.frame_file)?;
        if Some(identity) != self.owned_inode || current != sequence {
            return Err("Producer PAUSE acknowledgment does not match owned frame state".into());
        }
        Ok(sequence)
    }

    pub fn resume(&mut self, timeout: Duration) -> Result<u64,String> {
        let sequence = self.command("RESUME", "RESUMED", timeout)?;
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if !self.is_alive()? { return Err("Producer exited after RESUME".into()); }
            if let Ok((current, identity)) = read_frame_header(&self.frame_file) {
                if Some(identity) != self.owned_inode { return Err("Frame file changed after RESUME".into()); }
                if current > sequence { return Ok(current); }
            }
            thread::sleep(Duration::from_millis(20));
        }
        Err("Producer acknowledged RESUME but did not publish another frame".into())
    }

    /// Reload the wallpaper renderer without restarting its OpenGL context.
    /// A rejected reload leaves the producer running its previous engine.
    /// RELOADED is emitted after the renderer has committed its new engine.
    pub fn reload(&mut self, timeout: Duration) -> Result<u64, String> {
        if !self.is_alive()? { return Err("KDE producer exited before RELOAD".into()); }
        let input = self.control.as_mut().ok_or("Producer command pipe is closed")?;
        writeln!(input, "RELOAD").and_then(|_| input.flush())
            .map_err(|e| format!("Cannot send RELOAD to KDE producer: {e}"))?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err("Producer RELOAD acknowledgment timed out".into()); }
            match self.acknowledgments.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(line) => {
                    if let Some(sequence) = parse_control_ack(&line, "RELOADED") {
                        let (_, identity) = read_frame_header(&self.frame_file)?;
                        if Some(identity) != self.owned_inode {
                            return Err("Frame file identity changed during RELOAD".into());
                        }
                        return Ok(sequence);
                    }
                    if line.starts_with("[PLASMA CONTROL] RELOAD_FAILED") {
                        return Err(format!("KDE renderer rejected policy reload: {line}"));
                    }
                    if line.starts_with("[PLASMA CONTROL] UNSUPPORTED") {
                        return Err(format!("KDE producer does not support RELOAD: {line}"));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !self.is_alive()? { return Err("Producer exited awaiting RELOAD acknowledgment".into()); }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Producer acknowledgment stream closed".into()),
            }
        }
    }

    pub fn frame_file(&self) -> &Path { &self.frame_file }
    pub fn pid(&self) -> u32 { self.child.id() }

    pub fn is_alive(&mut self) -> Result<bool,String> {
        self.child.try_wait().map(|s| s.is_none()).map_err(|e| format!("Producer status: {e}"))
    }

    /// Confirm two distinct published frame sequences from the same file inode.
    pub fn wait_for_frames(&mut self, timeout: Duration) -> Result<(),String> {
        let deadline = Instant::now() + timeout;
        let mut previous: Option<u64> = None;
        while Instant::now() < deadline {
            if !self.is_alive()? { return Err("KDE frame producer exited before readiness".into()); }
            if let Ok((seq,identity)) = read_frame_header(&self.frame_file) {
                match self.owned_inode {
                    None => self.owned_inode = Some(identity),
                    Some(owned) if owned != identity => return Err("Producer frame inode changed unexpectedly".into()),
                    _ => {}
                }
                if previous.is_some_and(|old| seq > old) { return Ok(()); }
                previous = Some(seq);
            }
            thread::sleep(Duration::from_millis(40));
        }
        Err("KDE frame producer did not publish advancing frames within 12 seconds".into())
    }

    /// Check producer health without inferring that Plasma has displayed a frame.
    pub fn verify_progress(&mut self, previous: u64) -> Result<u64,String> {
        if !self.is_alive()? { return Err("KDE frame producer exited".into()); }
        let (sequence,identity) = read_frame_header(&self.frame_file)?;
        if Some(identity) != self.owned_inode { return Err("KDE frame file identity changed".into()); }
        if sequence <= previous { return Err("KDE producer frame sequence did not advance".into()); }
        Ok(sequence)
    }

    pub fn stop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            if let Some(mut input) = self.control.take() {
                let _ = writeln!(input, "STOP");
                let _ = input.flush();
            }
            // Closing stdin also requests STOP if the command was not delivered.
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                match self.child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => thread::sleep(Duration::from_millis(25)),
                    Err(_) => break,
                }
            }
            if matches!(self.child.try_wait(), Ok(None)) { let _ = self.child.kill(); }
            let _ = self.child.wait();
        }
        // SIGKILL prevents the producer's RAII cleanup. Remove only our exact inode,
        // and only after the child has been reaped. Never remove another session's file.
        if let (Some((dev,ino)),Ok(meta)) = (self.owned_inode,fs::symlink_metadata(&self.frame_file)) {
            if meta.file_type().is_file() && meta.dev()==dev && meta.ino()==ino {
                let _ = fs::remove_file(&self.frame_file);
            }
        }
    }
}
impl Drop for ProducerSupervisor { fn drop(&mut self) { self.stop(); } }

fn read_frame_header(path: &Path) -> Result<(u64,(u64,u64)),String> {
    let mut file = fs::File::open(path).map_err(|e| format!("Frame open: {e}"))?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() { return Err("Frame transport is not a regular file".into()); }
    let mut header = [0u8;HEADER_SIZE];
    file.read_exact(&mut header).map_err(|e| format!("Frame header: {e}"))?;
    if &header[..8] != MAGIC || u32::from_le_bytes(header[8..12].try_into().unwrap()) != 1
        || u32::from_le_bytes(header[12..16].try_into().unwrap()) != 64 {
        return Err("Invalid KDE frame transport header".into());
    }
    let sequence = u64::from_le_bytes(header[16..24].try_into().unwrap());
    if sequence == 0 { return Err("Producer has not published its first frame".into()); }
    let width = u32::from_le_bytes(header[24..28].try_into().unwrap()) as u64;
    let height = u32::from_le_bytes(header[28..32].try_into().unwrap()) as u64;
    let stride = u32::from_le_bytes(header[32..36].try_into().unwrap()) as u64;
    let bytes = u32::from_le_bytes(header[36..40].try_into().unwrap()) as u64;
    if width == 0 || height == 0 || width > 16384 || height > 16384
        || stride != width*4 || bytes != stride*height || meta.len() < 64+2*bytes {
        return Err("Invalid KDE frame dimensions or file length".into());
    }
    Ok((sequence,(meta.dev(),meta.ino())))
}

fn parse_control_ack(line: &str, expected: &str) -> Option<u64> {
    let rest = line.strip_prefix("[PLASMA CONTROL] ")?;
    let (state, value) = rest.split_once(" sequence=")?;
    if state != expected { return None; }
    value.trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_control_ack;
    #[test]
    fn acknowledges_exact_state_and_sequence() {
        assert_eq!(parse_control_ack("[PLASMA CONTROL] PAUSED sequence=42", "PAUSED"), Some(42));
        assert_eq!(parse_control_ack("[PLASMA CONTROL] RESUMED sequence=43", "RESUMED"), Some(43));
        assert_eq!(parse_control_ack("[PLASMA CONTROL] PAUSED sequence=42", "RESUMED"), None);
        assert_eq!(parse_control_ack("[PLASMA CONTROL] PAUSED sequence=bad", "PAUSED"), None);
        assert_eq!(parse_control_ack("[PLASMA CONTROL] RELOADED sequence=51", "RELOADED"), Some(51));
        assert_eq!(parse_control_ack("[PLASMA CONTROL] RELOAD_FAILED sequence=51 reason=\"invalid\"", "RELOADED"), None);
    }
}
