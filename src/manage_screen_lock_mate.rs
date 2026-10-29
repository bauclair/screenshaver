//! Native MATE screensaver lock coordination. MATE owns authentication and unlock.

use std::path::Path;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::time::{
    Duration,
    Instant,
};

use zbus::blocking::{
    Connection,
    Proxy,
};


const MATE_SCREENSAVER_DESTINATION: &str =
    "org.mate.ScreenSaver";

const MATE_SCREENSAVER_PATH: &str =
    "/org/mate/ScreenSaver";

const MATE_SCREENSAVER_INTERFACE: &str =
    "org.mate.ScreenSaver";

const LOCK_CONFIRMATION_TIMEOUT: Duration =
    Duration::from_secs(5);

const LOCK_STATE_POLL_INTERVAL: Duration =
    Duration::from_millis(100);


pub fn run(
    logfile: &Path,
    running: &AtomicBool,
    wallpaper_control:
        &crate::manage_wallpaper_runtime::PresentationRuntimeControl,
) -> Result<(), String> {

    crate::logger::information(
        logfile,
        "[LOCK] MATE secure-lock backend selected",
    );

    let connection =
        Connection::session()
            .map_err(
                |error| {
                    format!(
                        "Unable to connect to the MATE session bus: {}",
                        error,
                    )
                }
            )?;

    let proxy =
        Proxy::new(
            &connection,
            MATE_SCREENSAVER_DESTINATION,
            MATE_SCREENSAVER_PATH,
            MATE_SCREENSAVER_INTERFACE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create MATE ScreenSaver D-Bus proxy: {}",
                    error,
                )
            }
        )?;

    if !wallpaper_control.request_pause_after_first_frame(
        running
    ) {
        return Err(
            "Wallpaper renderer did not acknowledge pause before MATE screen lock"
                .to_string()
        );
    }

    crate::logger::information(
        logfile,
        "[LOCK] Requesting MATE screen lock",
    );

    let _: () =
        proxy
            .call(
                "Lock",
                &(),
            )
            .map_err(
                |error| {
                    format!(
                        "MATE screen-lock request failed: {}",
                        error,
                    )
                }
            )?;

    let confirmation_deadline =
        Instant::now()
            + LOCK_CONFIRMATION_TIMEOUT;

    loop {
        let active =
            get_active(
                &proxy
            )?;

        if active {
            break;
        }

        if Instant::now()
            >= confirmation_deadline
        {
            return Err(
                format!(
                    "MATE did not confirm an active screen lock within {} seconds",
                    LOCK_CONFIRMATION_TIMEOUT.as_secs(),
                )
            );
        }

        std::thread::sleep(
            LOCK_STATE_POLL_INTERVAL
        );
    }

    crate::logger::information(
        logfile,
        "[LOCK] MATE screen lock confirmed active",
    );

    crate::logger::information(
        logfile,
        "[LOCK] Waiting for MATE authenticated unlock",
    );

    let mut shutdown_deferred_logged =
        false;

    loop {
        if !running.load(
            Ordering::SeqCst
        ) && !shutdown_deferred_logged
        {
            crate::logger::information(
                logfile,
                "[LOCK] Shutdown requested while MATE screen lock is active; deferring Screenshaver shutdown until MATE unlocks",
            );

            shutdown_deferred_logged =
                true;
        }

        if !get_active(
            &proxy
        )? {
            break;
        }

        std::thread::sleep(
            LOCK_STATE_POLL_INTERVAL
        );
    }

    crate::logger::information(
        logfile,
        "[LOCK] MATE screen lock released",
    );

    if running.load(
        Ordering::SeqCst
    ) {
        wallpaper_control.resume_and_wait_for_frame(
            running
        );
    } else {
        crate::logger::information(
            logfile,
            "[LOCK] MATE unlock completed with shutdown pending; wallpaper renderer will remain stopped",
        );
    }

    Ok(())
}


fn get_active(
    proxy: &Proxy<'_>,
) -> Result<bool, String> {

    proxy
        .call(
            "GetActive",
            &(),
        )
        .map_err(
            |error| {
                format!(
                    "Unable to query MATE screen-lock state: {}",
                    error,
                )
            }
        )
}
