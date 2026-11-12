//! Native MATE lock coordination. MATE exclusively owns authentication.
//!
//! Important: a blocking D-Bus `Lock` method call can remain unanswered after
//! authentication. Send the method call with no reply requested, then observe
//! the independent `GetActive` state transition on the session bus.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use zbus::blocking::{Connection, Proxy};

const DESTINATION: &str = "org.mate.ScreenSaver";
const OBJECT_PATH: &str = "/org/mate/ScreenSaver";
const INTERFACE: &str = "org.mate.ScreenSaver";
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

pub fn run(
    logfile: &Path,
    running: &AtomicBool,
    wallpaper_control: &crate::manage_wallpaper_runtime::PresentationRuntimeControl,
) -> Result<(), String> {
    crate::logger::information(logfile, "[LOCK] MATE secure-lock backend selected");

    let connection = Connection::session()
        .map_err(|error| format!("Unable to connect to MATE session bus: {error}"))?;
    let proxy = Proxy::new(&connection, DESTINATION, OBJECT_PATH, INTERFACE)
        .map_err(|error| format!("Unable to create MATE ScreenSaver D-Bus proxy: {error}"))?;

    // Establish a known initial state before requesting a new lock. An
    // initial false must never be mistaken for a completed authentication.
    match get_active(&proxy) {
        Ok(false) => {}
        Ok(true) => {
            return Err("MATE screen lock was already active before Screenshaver requested it".into());
        }
        Err(error) => return Err(format!("Unable to verify MATE pre-lock state: {error}")),
    }

    if !wallpaper_control.request_pause_after_first_frame(running) {
        return Err("Desktop renderers did not acknowledge pause before MATE screen lock".into());
    }

    crate::logger::information(logfile, "[LOCK] Requesting MATE screen lock (no D-Bus reply wait)");

    // dbus-send, without --print-reply, sends a METHOD_CALL with the
    // NO_REPLY_EXPECTED flag. Its process exits after sending the request;
    // it cannot hold the lock-monitoring worker waiting for MATE's reply.
    // This requires the standard dbus-send utility from dbus-tools.
    let send_result = Command::new("dbus-send")
        .arg("--session")
        .arg("--type=method_call")
        .arg(format!("--dest={DESTINATION}"))
        .arg(OBJECT_PATH)
        .arg(format!("{INTERFACE}.Lock"))
        .output();

    match send_result {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let error = String::from_utf8_lossy(&output.stderr);
            // Failure to send does not prove MATE did not engage; verify
            // state before allowing desktop presentation to resume.
            crate::logger::warning(logfile, &format!(
                "[LOCK] MATE no-reply Lock dispatch failed ({}): {}; verifying lock state",
                output.status, error.trim()
            ));
        }
        Err(error) => {
            crate::logger::warning(logfile, &format!(
                "[LOCK] Unable to execute dbus-send: {error}; verifying lock state"
            ));
        }
    }

    let confirmation_deadline = Instant::now() + CONFIRMATION_TIMEOUT;
    loop {
        match get_active(&proxy) {
            Ok(true) => {
                break;
            }
            Ok(false) => {
                if Instant::now() >= confirmation_deadline {
                    // Native state is confirmed inactive. No authentication
                    // boundary is being bypassed by restoring presentation.
                    if running.load(Ordering::SeqCst) {
                        wallpaper_control.resume_and_wait_for_frame(running);
                    }
                    return Err("MATE did not confirm lock activation within 5 seconds".into());
                }
            }
            Err(error) => {
                crate::logger::warning(logfile, &format!(
                    "[LOCK] MATE activation state unavailable: {error}; retaining renderer pause"
                ));
                // Unknown native state is not evidence of an unlocked desktop.
            }
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    crate::logger::information(logfile, "[LOCK] MATE screen lock confirmed active");
    crate::logger::information(logfile, "[LOCK] Waiting for MATE authenticated unlock");

    let mut shutdown_deferred_logged = false;
    loop {
        if !running.load(Ordering::SeqCst) && !shutdown_deferred_logged {
            crate::logger::information(logfile,
                "[LOCK] Shutdown requested during native lock; deferring until authenticated unlock");
            shutdown_deferred_logged = true;
        }
        match get_active(&proxy) {
            Ok(false) => break,
            Ok(true) => {}
            Err(error) => {
                crate::logger::warning(logfile, &format!(
                    "[LOCK] MATE unlock state unavailable: {error}; keeping desktop presentation paused"
                ));
            }
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    crate::logger::information(logfile, "[LOCK] MATE screen lock released");
    if running.load(Ordering::SeqCst) {
        if !wallpaper_control.resume_and_wait_for_frame(running) {
            crate::logger::warning(logfile,
                "[LOCK] MATE unlocked, but desktop renderer frame restoration was not confirmed");
        }
    } else {
        crate::logger::information(logfile,
            "[LOCK] MATE unlocked with shutdown pending; desktop renderers remain stopped");
    }
    Ok(())
}

fn get_active(proxy: &Proxy<'_>) -> Result<bool, String> {
    proxy.call("GetActive", &())
        .map_err(|error| format!("Unable to query MATE screen-lock state: {error}"))
}
