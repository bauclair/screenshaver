use std::collections::HashMap;
use std::sync::mpsc::{
    self,
    Receiver,
};
use std::thread;

use crate::fps_monitor::FpsWarningState;


#[derive(
    Debug,
    Clone,
    PartialEq,
)]
pub struct WallpaperMetadata {

    pub wallpaper: String,

    pub animation_speed: f32,

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
                    "Shader: {} ({})",
                    self.wallpaper,
                    format_animation_speed(
                        self.animation_speed
                    ),
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


pub struct WallpaperNotificationState {
    active_critical_id:
        Option<u32>,

    closed_notifications:
        Receiver<u32>,
}


impl WallpaperNotificationState {

    pub fn new() -> Self {

        Self {
            active_critical_id:
                None,

            closed_notifications:
                spawn_notification_closed_listener(),
        }
    }


    fn refresh_closed_notification_state(
        &mut self,
    ) {

        while let Ok(notification_id) =
            self.closed_notifications
                .try_recv()
        {
            if self.active_critical_id
                == Some(notification_id)
            {
                self.active_critical_id =
                    None;
            }
        }
    }


    pub fn show_shader_changed(
        &mut self,
        enabled: bool,
        metadata: &WallpaperMetadata,
    ) {

        self.refresh_closed_notification_state();


        self.close_active_critical();


        let _ =
            show(
                enabled,
                metadata,
            );
    }


    pub fn show_update(
        &mut self,
        enabled: bool,
        metadata: &WallpaperMetadata,
    ) {

        self.refresh_closed_notification_state();


        if self.active_critical_id
            .is_some()
            && metadata.is_performance_alert()
        {
            return;
        }


        let notification_id =
            show(
                enabled,
                metadata,
            );


        if metadata.is_critical() {
            self.active_critical_id =
                notification_id;
        }
    }


    pub fn close_active_critical(
        &mut self,
    ) {

        self.refresh_closed_notification_state();


        if let Some(notification_id) =
            self.active_critical_id
                .take()
        {
            close(
                notification_id
            );
        }
    }
}


impl Default
    for WallpaperNotificationState
{
    fn default() -> Self {

        Self::new()
    }
}


impl Drop
    for WallpaperNotificationState
{
    fn drop(
        &mut self,
    ) {

        self.close_active_critical();
    }
}


fn spawn_notification_closed_listener(
) -> Receiver<u32> {

    let (
        sender,
        receiver,
    ) =
        mpsc::channel();


    thread::spawn(
        move || {

            let connection =
                match zbus::blocking::Connection::session() {
                    Ok(connection) => {
                        connection
                    }

                    Err(error) => {
                        eprintln!(
                            "[WALLPAPER] Notification close listener unavailable: {}",
                            error,
                        );

                        return;
                    }
                };


            let proxy =
                match zbus::blocking::Proxy::new(
                    &connection,
                    "org.freedesktop.Notifications",
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.Notifications",
                ) {
                    Ok(proxy) => {
                        proxy
                    }

                    Err(error) => {
                        eprintln!(
                            "[WALLPAPER] Could not create notification signal proxy: {}",
                            error,
                        );

                        return;
                    }
                };


            let mut signals =
                match proxy.receive_signal(
                    "NotificationClosed"
                ) {
                    Ok(signals) => {
                        signals
                    }

                    Err(error) => {
                        eprintln!(
                            "[WALLPAPER] Could not listen for NotificationClosed: {}",
                            error,
                        );

                        return;
                    }
                };


            for message in
                &mut signals
            {
                let body =
                    message.body();


                let Ok((
                    notification_id,
                    _reason,
                )) =
                    body.deserialize::<(
                        u32,
                        u32,
                    )>()
                else {
                    continue;
                };


                if sender.send(
                    notification_id
                )
                .is_err()
                {
                    break;
                }
            }
        }
    );


    receiver
}


fn format_animation_speed(
    speed: f32,
) -> String {

    if speed.fract()
        == 0.0
    {
        format!(
            "×{speed:.1}"
        )
    } else {
        format!(
            "×{speed}"
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

