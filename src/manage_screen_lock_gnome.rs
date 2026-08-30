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


const GNOME_SCREENSAVER_DESTINATION: &str =
    "org.gnome.ScreenSaver";

const GNOME_SCREENSAVER_PATH: &str =
    "/org/gnome/ScreenSaver";

const GNOME_SCREENSAVER_INTERFACE: &str =
    "org.gnome.ScreenSaver";

const LOCK_CONFIRMATION_TIMEOUT: Duration =
    Duration::from_secs(5);

const LOCK_STATE_POLL_INTERVAL: Duration =
    Duration::from_millis(100);


pub fn run(
    logfile: &Path,
    running: &AtomicBool,
    wallpaper_control:
        &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
) -> Result<(), String> {

    crate::logger::information(
        logfile,
        "[LOCK] GNOME secure-lock backend selected",
    );


    let connection =
        Connection::session()
            .map_err(
                |error| {
                    format!(
                        "Unable to connect to the GNOME session bus: {}",
                        error,
                    )
                }
            )?;


    let proxy =
        Proxy::new(
            &connection,
            GNOME_SCREENSAVER_DESTINATION,
            GNOME_SCREENSAVER_PATH,
            GNOME_SCREENSAVER_INTERFACE,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create GNOME ScreenSaver D-Bus proxy: {}",
                    error,
                )
            }
        )?;


    if !wallpaper_control.request_pause_after_first_frame(
        running
    ) {
        return Err(
            "Wallpaper renderer did not acknowledge pause before GNOME screen lock"
                .to_string()
        );
    }


    crate::logger::information(
        logfile,
        "[LOCK] Requesting GNOME Shell screen lock",
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
                        "GNOME Shell screen-lock request failed: {}",
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
                    "GNOME Shell did not confirm an active screen lock within {} seconds",
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
        "[LOCK] GNOME screen lock confirmed active",
    );


    crate::logger::information(
        logfile,
        "[LOCK] Waiting for GNOME authenticated unlock",
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
                "[LOCK] Shutdown requested while GNOME screen lock is active; deferring Screenshaver shutdown until GNOME unlocks",
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
        "[LOCK] GNOME screen lock released",
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
            "[LOCK] GNOME unlock completed with shutdown pending; wallpaper renderer will remain stopped",
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
                    "Unable to query GNOME screen-lock state: {}",
                    error,
                )
            }
        )
}
