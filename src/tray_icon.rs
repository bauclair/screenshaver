use std::sync::mpsc::Sender;

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use ksni::{Category, MenuItem, Status, ToolTip, Tray};

const APPLICATION_ID: &str = "screenshaver";
const APPLICATION_NAME: &str = "Screenshaver";
const ICON_NAME: &str = "screenshaver";
const TOOLTIP_DESCRIPTION: &str = "Waiting for idle...";

/// Commands that can be requested through the system tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Restart,
    Stop,
}

/// Immutable runtime status displayed by the system tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayStatus {
    pub screensaver_enabled: bool,
    pub wallpaper_enabled: bool,
}

/// The Status Notifier Item presented to the desktop panel.
#[derive(Debug)]
pub struct ScreenshaverTray {
    command_sender: Sender<TrayCommand>,
    status: TrayStatus,
}

/// Handle retained by `main.rs` for the lifetime of Screenshaver.
///
/// Dropping this handle does not need to be part of normal application logic;
/// retaining it simply keeps the tray service associated with the process.
pub type TrayHandle = Handle<ScreenshaverTray>;

impl ScreenshaverTray {
    fn new(
        command_sender: Sender<TrayCommand>,
        status: TrayStatus,
    ) -> Self {
        Self {
            command_sender,
            status,
        }
    }

    fn send_command(&self, command: TrayCommand) {
        // The receiving side may already have shut down. That is harmless:
        // the application is already exiting, so there is nothing left to do.
        let _ = self.command_sender.send(command);
    }
}

impl Tray for ScreenshaverTray {
    /// Ask the host to open the menu when the tray icon is activated.
    ///
    /// Screenshaver does not have a primary tray-icon action, so opening the
    /// menu is the most useful response to either left- or right-clicking it.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        APPLICATION_ID.into()
    }

    fn title(&self) -> String {
        APPLICATION_NAME.into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn icon_name(&self) -> String {
        ICON_NAME.into()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: ICON_NAME.into(),
            title: APPLICATION_NAME.into(),
            description: TOOLTIP_DESCRIPTION.into(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: format!(
                    "Screensaver: {}",
                    enabled_status(self.status.screensaver_enabled),
                ),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Wallpaper: {}",
                    enabled_status(self.status.wallpaper_enabled),
                ),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Restart".into(),
                icon_name: "view-refresh".into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.send_command(TrayCommand::Restart);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Stop".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.send_command(TrayCommand::Stop);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Attempts to register the Screenshaver Status Notifier Item.
///
/// The caller must treat an error as non-fatal. Screenshaver's idle monitoring,
/// rendering, and command-line controls must continue to work when the active
/// desktop session does not provide tray-icon support.
pub fn start(
    command_sender: Sender<TrayCommand>,
    status: TrayStatus,
) -> Result<TrayHandle, ksni::Error> {
    ScreenshaverTray::new(
        command_sender,
        status,
    )
    .spawn()
}

fn enabled_status(enabled: bool) -> &'static str {
    if enabled {
        "Enabled"
    } else {
        "Disabled"
    }
}

