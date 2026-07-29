use std::collections::HashMap;

use crate::fps_monitor::FpsWarningState;


#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct WallpaperMetadata {

    pub wallpaper: String,

    pub texture: Option<String>,

    pub palette: Option<String>,

    pub fps: u32,

    pub warning_state: FpsWarningState,
}


impl WallpaperMetadata {

    pub fn body(
        &self,
    ) -> String {

        let mut lines =
            vec![
                format!(
                    "Wallpaper: {}",
                    self.wallpaper,
                )
            ];


        match self.warning_state {
            FpsWarningState::Normal => {}

            FpsWarningState::Warning => {
                lines.push(
                    "Performance Warning".to_string()
                );
            }

            FpsWarningState::Critical
            | FpsWarningState::CriticalHidden => {
                lines.push(
                    "Performance CRITICAL".to_string()
                );
            }
        }


        lines.push(
            format!(
                "FPS: {}",
                self.fps.max(1),
            )
        );


        if let Some(texture) =
            &self.texture
        {
            lines.push(
                format!(
                    "Texture: {}",
                    texture,
                )
            );
        }


        if let Some(palette) =
            &self.palette
        {
            lines.push(
                format!(
                    "Palette: {}",
                    palette,
                )
            );
        }


        lines.join(
            "\n"
        )
    }


    pub fn is_performance_alert(
        &self,
    ) -> bool {

        matches!(
            self.warning_state,
            FpsWarningState::Warning
                | FpsWarningState::Critical
                | FpsWarningState::CriticalHidden
        )
    }


    pub fn is_critical(
        &self,
    ) -> bool {

        matches!(
            self.warning_state,
            FpsWarningState::Critical
                | FpsWarningState::CriticalHidden
        )
    }
}


/// Displays a wallpaper notification.
///
/// Normal wallpaper-change notifications obey `enabled`.
/// Performance Warning and Performance CRITICAL notifications bypass
/// `enabled`, so performance alerts are never suppressed by the general
/// wallpaper notification setting.
///
/// Returns the notification ID assigned by the desktop notification daemon.
/// Wallpaper mode should retain this ID only for a CRITICAL notification and
/// pass it to `close()` when the shader stops rendering.
pub fn show(
    enabled: bool,
    metadata: &WallpaperMetadata,
) -> Option<u32> {

    if !enabled
        && !metadata.is_performance_alert()
    {
        return None;
    }


    match send_notification(
        "Screenshaver Wallpaper",
        &metadata.body(),
        metadata.is_critical(),
    ) {
        Ok(notification_id) => {
            Some(notification_id)
        }

        Err(error) => {
            eprintln!(
                "[WALLPAPER] Notification unavailable: {}",
                error
            );

            None
        }
    }
}


/// Closes a notification previously returned by `show()`.
///
/// Call this when the shader represented by a persistent CRITICAL
/// notification stops rendering. Notification-daemon errors are logged but
/// are not fatal to wallpaper rendering.
pub fn close(
    notification_id: u32,
) {

    if let Err(error) =
        close_notification(
            notification_id,
        )
    {
        eprintln!(
            "[WALLPAPER] Could not close notification {}: {}",
            notification_id,
            error,
        );
    }
}


fn send_notification(
    summary: &str,
    body: &str,
    critical: bool,
) -> Result<u32, String> {

    let connection =
        zbus::blocking::Connection::session()
            .map_err(
                |error| {
                    format!(
                        "could not connect to the session D-Bus: {}",
                        error
                    )
                }
            )?;


    let actions:
        Vec<&str> =
        Vec::new();


    let mut hints:
        HashMap<
            &str,
            zbus::zvariant::Value<'_>,
        > =
        HashMap::new();


    if critical {
        hints.insert(
            "urgency",
            zbus::zvariant::Value::U8(
                2,
            ),
        );
    }


    // A zero timeout requests that CRITICAL notifications remain visible
    // until explicitly dismissed or closed. Non-critical notifications use
    // the existing five-second timeout.
    let expire_timeout =
        if critical {
            0_i32
        } else {
            5000_i32
        };


    let reply =
        connection
            .call_method(
                Some(
                    "org.freedesktop.Notifications"
                ),
                "/org/freedesktop/Notifications",
                Some(
                    "org.freedesktop.Notifications"
                ),
                "Notify",
                &(
                    "screenshaver",
                    0_u32,
                    "Screenshaver",
                    summary,
                    body,
                    actions,
                    hints,
                    expire_timeout,
                ),
            )
            .map_err(
                |error| {
                    format!(
                        "D-Bus notification failed: {}",
                        error
                    )
                }
            )?;


    reply
        .body()
        .deserialize::<u32>()
        .map_err(
            |error| {
                format!(
                    "could not read notification ID: {}",
                    error
                )
            }
        )
}


fn close_notification(
    notification_id: u32,
) -> Result<(), String> {

    let connection =
        zbus::blocking::Connection::session()
            .map_err(
                |error| {
                    format!(
                        "could not connect to the session D-Bus: {}",
                        error
                    )
                }
            )?;


    connection
        .call_method(
            Some(
                "org.freedesktop.Notifications"
            ),
            "/org/freedesktop/Notifications",
            Some(
                "org.freedesktop.Notifications"
            ),
            "CloseNotification",
            &(
                notification_id,
            ),
        )
        .map_err(
            |error| {
                format!(
                    "D-Bus CloseNotification failed: {}",
                    error
                )
            }
        )?;


    Ok(())
}

