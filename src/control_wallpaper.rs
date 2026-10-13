//! Cross-process wallpaper pause/resume coordination.
//! A transaction lock serializes requesters; acknowledgment files echo the
//! current request token. Never treat an unverified acknowledgment as success.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use fs2::FileExt;

const CONTROL_WAIT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(2);
const ACTIVE_FILE: &str = "screenshaver-wallpaper.active";
const PAUSE_REQUEST_FILE: &str = "screenshaver-wallpaper.pause";
const PAUSE_ACK_FILE: &str = "screenshaver-wallpaper.paused";
const RESUME_ACK_FILE: &str = "screenshaver-wallpaper.resumed";
const RESUME_TOKEN_FILE: &str = "screenshaver-wallpaper.resume-token";
const TRANSACTION_LOCK_FILE: &str = "screenshaver-wallpaper.transaction.lock";
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);
// Set only by the early-exit developer diagnostic, before any runtime starts.
static TEST_RUNTIME_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

pub struct WallpaperPauseGuard {
    token: Option<String>,
    // The lock is held until resume/cancellation, preventing another preview
    // from withdrawing this request or consuming its acknowledgment.
    _transaction_lock: Option<File>,
}

impl WallpaperPauseGuard {
    pub fn acquire() -> Result<Self, String> {
        let lock_path = required_path(TRANSACTION_LOCK_FILE)?;
        let lock = OpenOptions::new().read(true).write(true).create(true)
            .open(&lock_path).map_err(|e| format!("Cannot open wallpaper transaction lock: {e}"))?;
        lock.try_lock_exclusive().map_err(|e| format!("Wallpaper pause transaction already owned: {e}"))?;
        let token = request_pause()?;
        Ok(Self { token, _transaction_lock: Some(lock) })
    }
}

impl Drop for WallpaperPauseGuard {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            resume_and_wait_for_frame(&token);
        }
    }
}

pub(crate) fn external_pause_requested() -> bool {
    match read_marker(PAUSE_REQUEST_FILE) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(error) => {
            log_warning(&format!("Cannot inspect external pause request: {error}"));
            true // A broken channel must not be interpreted as permission to render.
        }
    }
}

/// Snapshot the transaction at pause detection, not at acknowledgment time.
/// A late worker must never acknowledge a replacement request.
pub(crate) fn detected_pause_token() -> Result<Option<String>, String> {
    read_marker(PAUSE_REQUEST_FILE)
}

pub(crate) fn acknowledge_paused_for(token: &str) {
    match read_marker(PAUSE_REQUEST_FILE) {
        Ok(Some(current)) if current == token => {
            if let Err(error) = write_marker(PAUSE_ACK_FILE, token) {
                log_warning(&format!("Unable to publish pause acknowledgment: {error}"));
            }
        }
        Ok(_) => log_warning("Refusing stale pause acknowledgment: transaction changed"),
        Err(error) => log_warning(&format!("Cannot verify pause transaction: {error}")),
    }
}

pub(crate) fn acknowledge_resumed_frame_for(token: &str) {
    // A frame must be associated with the previously observed pause, and the
    // request must have been withdrawn before it counts as resumed.
    match (read_marker(PAUSE_REQUEST_FILE), read_marker(RESUME_TOKEN_FILE)) {
        (Ok(None), Ok(Some(current))) if current == token => {
            if let Err(error) = write_marker(RESUME_ACK_FILE, token) {
                log_warning(&format!("Unable to publish resume acknowledgment: {error}"));
            }
        }
        (Ok(_), Ok(_)) => {},
        (Err(error), _) | (_, Err(error)) =>
            log_warning(&format!("Cannot verify resume transaction: {error}")),
    }
}

pub(crate) fn set_runtime_active(active: bool) {
    if active {
        for name in [PAUSE_ACK_FILE, RESUME_ACK_FILE, RESUME_TOKEN_FILE] {
            if let Err(e) = remove_marker(name) { log_warning(&format!("Cannot reset {name}: {e}")); }
        }
        if let Err(e) = write_marker(ACTIVE_FILE, &std::process::id().to_string()) {
            log_warning(&format!("Cannot publish wallpaper active state: {e}"));
        }
    } else {
        for name in [ACTIVE_FILE, PAUSE_ACK_FILE, RESUME_ACK_FILE, RESUME_TOKEN_FILE] {
            if let Err(e) = remove_marker(name) { log_warning(&format!("Cannot remove {name}: {e}")); }
        }
    }
}

fn new_token() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    format!("{}-{nanos}-{}", std::process::id(), NEXT_TOKEN.fetch_add(1, Ordering::Relaxed))
}

fn request_pause() -> Result<Option<String>, String> {
    if !runtime_is_active()? { return Ok(None); }
    let token = new_token();
    remove_marker(PAUSE_ACK_FILE)?;
    remove_marker(RESUME_ACK_FILE)?;
    remove_marker(RESUME_TOKEN_FILE)?;
    write_marker(PAUSE_REQUEST_FILE, &token)?;
    let acknowledged = wait_for_token(PAUSE_ACK_FILE, &token);
    if matches!(acknowledged, Ok(true)) { return Ok(Some(token)); }
    // Do not remove a request that another actor has replaced.
    remove_if_matching(PAUSE_REQUEST_FILE, &token)?;
    if let Err(error) = acknowledged { return Err(error); }
    if !runtime_is_active()? { return Ok(None); }
    let message = format!("Timed out after {} ms waiting for matching wallpaper pause acknowledgment; refusing competing renderer", CONTROL_WAIT.as_millis());
    log_warning(&message);
    Err(message)
}

fn resume_and_wait_for_frame(token: &str) {
    if let Err(error) = write_marker(RESUME_TOKEN_FILE, token) {
        log_warning(&format!("Cannot register resume transaction: {error}"));
    }
    if let Err(error) = remove_if_matching(PAUSE_REQUEST_FILE, token) {
        log_warning(&format!("Cannot withdraw pause transaction: {error}"));
        return;
    }
    match runtime_is_active() {
        Ok(true) => match wait_for_token(RESUME_ACK_FILE, token) {
            Ok(true) => {},
            Ok(false) => log_warning("Resume presentation not confirmed within deadline; compositor may be withholding frames"),
            Err(e) => log_warning(&format!("Resume acknowledgment unavailable: {e}")),
        },
        Ok(false) => {},
        Err(e) => log_warning(&format!("Cannot verify wallpaper runtime during resume: {e}")),
    }
    let _ = remove_if_matching(RESUME_TOKEN_FILE, token);
}

fn runtime_is_active() -> Result<bool, String> {
    Ok(required_path(ACTIVE_FILE)?.exists())
}

fn wait_for_token(name: &str, token: &str) -> Result<bool, String> {
    let deadline = Instant::now() + CONTROL_WAIT;
    loop {
        if read_marker(name)?.as_deref() == Some(token) { return Ok(true); }
        if Instant::now() >= deadline { return Ok(false); }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn required_path(name: &str) -> Result<PathBuf, String> {
    Ok(runtime_directory()?.join(name))
}

fn read_marker(name: &str) -> Result<Option<String>, String> {
    let path = required_path(name)?;
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(Some(contents.trim().to_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read {}: {error}", path.display())),
    }
}

fn write_marker(name: &str, value: &str) -> Result<(), String> {
    let path = required_path(name)?;
    let sequence = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{value}").map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

fn remove_marker(name: &str) -> Result<(), String> {
    let path = required_path(name)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn remove_if_matching(name: &str, token: &str) -> Result<(), String> {
    if read_marker(name)?.as_deref() == Some(token) { remove_marker(name)?; }
    Ok(())
}

fn control_path(
    name: &str,
) -> Option<PathBuf> {

    match runtime_directory() {
        Ok(runtime_directory) => {
            Some(
                runtime_directory.join(
                    name
                )
            )
        }

        Err(error) => {
            log_warning(
                &error
            );

            None
        }
    }
}


fn runtime_directory() -> Result<PathBuf, String> {
    if let Some(path) = TEST_RUNTIME_DIRECTORY.get() {
        return Ok(path.clone());
    }

    static RUNTIME_DIRECTORY:
        OnceLock<Result<PathBuf, String>> =
            OnceLock::new();

    RUNTIME_DIRECTORY
        .get_or_init(
            resolve_runtime_directory
        )
        .clone()
}


fn resolve_runtime_directory() -> Result<PathBuf, String> {

    if let Some(runtime_directory) =
        std::env::var_os(
            "XDG_RUNTIME_DIR"
        )
    {
        let path =
            PathBuf::from(
                runtime_directory
            );

        if path.is_dir() {
            log_information(
                &format!(
                    "Using XDG runtime directory: {}",
                    path.display(),
                )
            );

            return Ok(
                path
            );
        }

        log_warning(
            &format!(
                "XDG_RUNTIME_DIR does not name an existing directory: {}",
                path.display(),
            )
        );
    }

    let effective_uid =
        unsafe {
            libc::geteuid()
        };

    let fallback =
        PathBuf::from(
            format!(
                "/run/user/{effective_uid}"
            )
        );

    if fallback.is_dir() {
        log_information(
            &format!(
                "XDG_RUNTIME_DIR unavailable; using fallback runtime directory: {}",
                fallback.display(),
            )
        );

        return Ok(
            fallback
        );
    }

    Err(
        format!(
            "Unable to locate a usable wallpaper control runtime directory; checked XDG_RUNTIME_DIR and {}",
            fallback.display(),
        )
    )
}


fn log_information(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::information(
        &logfile,
        &format!(
            "[WALLPAPER_CONTROL] {}",
            message
        ),
    );
}


fn log_warning(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();

    crate::logger::warning(
        &logfile,
        &format!(
            "[WALLPAPER_CONTROL] {}",
            message
        ),
    );
}


// This developer diagnostic runs in a separate Screenshaver invocation. Its
// OnceLock override is set before any production runtime initialization; no
// environment variables, real runtime markers, or OpenRGB devices are touched.
pub(crate) fn run_protocol_tests() -> Result<(), String> {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct TempDirectory(PathBuf);
    impl Drop for TempDirectory {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }
    let root = std::env::temp_dir().join(format!(
        "screenshaver-protocol-test-{}-{}", std::process::id(), new_token()
    ));
    fs::create_dir(&root).map_err(|e| format!("Cannot create isolated test directory: {e}"))?;
    let _cleanup = TempDirectory(root.clone());
    TEST_RUNTIME_DIRECTORY.set(root.clone())
        .map_err(|_| "Protocol test directory already initialized".to_string())?;
    println!("[WALLPAPER PROTOCOL TEST] Isolated directory: {}", root.display());

    fn check(label: &str, result: Result<(), String>, failures: &mut Vec<String>) {
        match result {
            Ok(()) => println!("[PASS] {label}"),
            Err(e) => { println!("[FAIL] {label}: {e}"); failures.push(format!("{label}: {e}")); }
        }
    }
    fn ensure(condition: bool, reason: &str) -> Result<(), String> {
        if condition { Ok(()) } else { Err(reason.to_string()) }
    }
    fn reset() -> Result<(), String> {
        for name in [ACTIVE_FILE, PAUSE_REQUEST_FILE, PAUSE_ACK_FILE,
                     RESUME_ACK_FILE, RESUME_TOKEN_FILE] { remove_marker(name)?; }
        Ok(())
    }
    // Worker imitates the wallpaper renderer's token capture, verified pause
    // acknowledgment, and frame-present acknowledgment. It never renders.
    fn worker(stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut held: Option<String> = None;
            while !stop.load(Ordering::Relaxed) {
                match detected_pause_token() {
                    Ok(Some(token)) => {
                        if held.is_none() {
                            held = Some(token.clone());
                            acknowledge_paused_for(&token);
                        }
                    }
                    Ok(None) => {
                        if let Some(token) = held.take() {
                            acknowledge_resumed_frame_for(&token);
                        }
                    }
                    Err(_) => break,
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    }
    let mut failures = Vec::new();

    check("No active wallpaper: no pause requested", (|| {
        reset()?;
        let guard = WallpaperPauseGuard::acquire()?;
        ensure(guard.token.is_none(), "Unexpected pause token")?;
        drop(guard);
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Unexpected pause marker")
    })(), &mut failures);

    check("Stale pause ACK rejected", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stale = "old-transaction";
        write_marker(PAUSE_REQUEST_FILE, "current-transaction")?;
        acknowledge_paused_for(stale);
        ensure(read_marker(PAUSE_ACK_FILE)?.is_none(), "Stale ACK was accepted")?;
        reset()
    })(), &mut failures);

    check("Timeout refuses renderer and withdraws own marker", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        write_marker(PAUSE_ACK_FILE, "stale-ack")?;
        let result = WallpaperPauseGuard::acquire();
        ensure(result.is_err(), "Missing pause ACK incorrectly granted permission")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Timed-out pause marker remains")?;
        reset()
    })(), &mut failures);

    check("Successful transaction and concurrent lock exclusion", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let guard = WallpaperPauseGuard::acquire()?;
            ensure(guard.token.is_some(), "No acknowledged transaction")?;
            ensure(WallpaperPauseGuard::acquire().is_err(), "Concurrent requester acquired lock")?;
            drop(guard);
            ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Pause marker remains after guard drop")
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    check("Abandoned requester lock and marker", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let lock = OpenOptions::new().read(true).write(true).create(true)
            .open(required_path(TRANSACTION_LOCK_FILE)?).map_err(|e| e.to_string())?;
        lock.try_lock_exclusive().map_err(|e| e.to_string())?;
        write_marker(PAUSE_REQUEST_FILE, "abandoned-token")?;
        drop(lock); // Simulate requester process exit: kernel releases its lock.
        ensure(read_marker(PAUSE_REQUEST_FILE)?.as_deref() == Some("abandoned-token"),
               "Abandoned marker unexpectedly disappeared")?;
        // A new requester may replace the abandoned request, but that does not
        // establish that the wallpaper has actually resumed rendering.
        let result = WallpaperPauseGuard::acquire();
        ensure(result.is_err(), "Unacknowledged replacement was incorrectly accepted")?;
        ensure(read_marker(PAUSE_REQUEST_FILE)?.is_none(), "Replacement marker not withdrawn")?;
        reset()
    })(), &mut failures);

    check("Repeated transactions with fresh identities", (|| {
        reset()?;
        write_marker(ACTIVE_FILE, "test-runtime")?;
        let stop = Arc::new(AtomicBool::new(false));
        let handle = worker(Arc::clone(&stop));
        let result = (|| {
            let first = WallpaperPauseGuard::acquire()?;
            let first_token = first.token.clone().ok_or("Missing first token")?;
            drop(first);
            // Wait for the worker to observe the resumed state before the
            // next request, avoiding a fabricated compositor race.
            std::thread::sleep(Duration::from_millis(10));
            let second = WallpaperPauseGuard::acquire()?;
            ensure(second.token.as_deref() != Some(&first_token), "Token reused")?;
            drop(second);
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        handle.join().map_err(|_| "Worker panicked".to_string())?;
        reset()?;
        result
    })(), &mut failures);

    if failures.is_empty() {
        println!("[WALLPAPER PROTOCOL TEST] All 6 tests passed.");
        Ok(())
    } else {
        Err(format!("{} of 6 tests failed: {}", failures.len(), failures.join("; ")))
    }
}
