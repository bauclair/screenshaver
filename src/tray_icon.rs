use std::path::{Path, PathBuf};
use std::sync::{
    mpsc::Sender,
    Arc,
    Mutex,
    RwLock,
};

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
    EditWallpaper,
    Restart,
    Stop,
}

/// Wallpaper state displayed by the system tray menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WallpaperTrayStatus {
    Disabled,
    Starting,
    Active(ActiveWallpaperInfo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWallpaperInfo {
    pub display_name: String,
    pub path: PathBuf,
}

impl ActiveWallpaperInfo {
    pub fn new(
        display_name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            display_name: display_name.into(),
            path: path.into(),
        }
    }
}

/// Cloneable controller shared with wallpaper renderers.
///
/// The tray reads this state whenever its menu is opened, so Wayland and X11
/// backends can publish the currently active wallpaper without depending on
/// KSNI or the tray implementation.
#[derive(Clone)]
pub struct TrayStatusControl {
    wallpaper_status: Arc<RwLock<WallpaperTrayStatus>>,
    tray_handle: Arc<Mutex<Option<TrayHandle>>>,
}

impl std::fmt::Debug for TrayStatusControl {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        formatter
            .debug_struct("TrayStatusControl")
            .field(
                "wallpaper_status",
                &self.wallpaper_label(),
            )
            .finish_non_exhaustive()
    }
}

impl TrayStatusControl {
    pub fn new(
        wallpaper_enabled: bool,
    ) -> Self {
        let wallpaper_status =
            if wallpaper_enabled {
                WallpaperTrayStatus::Starting
            } else {
                WallpaperTrayStatus::Disabled
            };

        Self {
            wallpaper_status:
                Arc::new(
                    RwLock::new(
                        wallpaper_status
                    )
                ),
            tray_handle:
                Arc::new(
                    Mutex::new(None)
                ),
        }
    }

    pub fn set_starting(
        &self,
    ) {
        self.set_wallpaper_status(
            WallpaperTrayStatus::Starting
        );
    }

    pub fn set_active(
        &self,
        display_name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) {
        self.set_wallpaper_status(
            WallpaperTrayStatus::Active(
                ActiveWallpaperInfo::new(
                    display_name,
                    path,
                )
            )
        );
    }

    pub fn active_wallpaper(
        &self,
    ) -> Option<ActiveWallpaperInfo> {
        let status = match self.wallpaper_status.read() {
            Ok(status) => status.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        match status {
            WallpaperTrayStatus::Active(info) => Some(info),
            WallpaperTrayStatus::Disabled
            | WallpaperTrayStatus::Starting => None,
        }
    }

    pub fn set_disabled(
        &self,
    ) {
        self.set_wallpaper_status(
            WallpaperTrayStatus::Disabled
        );
    }

    fn set_wallpaper_status(
        &self,
        status: WallpaperTrayStatus,
    ) {
        match self.wallpaper_status.write() {
            Ok(
                mut current_status
            ) => {
                *current_status =
                    status;
            }

            Err(
                poisoned
            ) => {
                *poisoned.into_inner() =
                    status;
            }
        }

        self.refresh_tray();
    }

    fn attach_handle(
        &self,
        handle: TrayHandle,
    ) {
        match self.tray_handle.lock() {
            Ok(mut tray_handle) => {
                *tray_handle = Some(handle);
            }

            Err(poisoned) => {
                *poisoned.into_inner() =
                    Some(handle);
            }
        }
    }

    fn refresh_tray(
        &self,
    ) {
        let handle =
            match self.tray_handle.lock() {
                Ok(tray_handle) => {
                    tray_handle.clone()
                }

                Err(poisoned) => {
                    poisoned.into_inner()
                        .clone()
                }
            };

        if let Some(handle) = handle {
            let _ = handle.update(
                |_tray| {}
            );
        }
    }

    fn wallpaper_label(
        &self,
    ) -> String {
        let status =
            match self.wallpaper_status.read() {
                Ok(status) => {
                    status.clone()
                }

                Err(poisoned) => {
                    poisoned.into_inner()
                        .clone()
                }
            };

        match status {
            WallpaperTrayStatus::Disabled => {
                "Disabled".to_string()
            }

            WallpaperTrayStatus::Starting => {
                "Starting...".to_string()
            }

            WallpaperTrayStatus::Active(info) => {
                info.display_name
            }
        }
    }
}

/// Runtime status displayed by the system tray menu.
#[derive(Debug, Clone)]
pub struct TrayStatus {
    pub screensaver_enabled: bool,
    pub wallpaper:
        TrayStatusControl,
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

    fn send_command(
        &self,
        command: TrayCommand,
    ) {
        // The receiving side may already have shut down. That is harmless:
        // the application is already exiting, so there is nothing left to do.
        let _ =
            self.command_sender.send(
                command
            );
    }
}

impl Tray for ScreenshaverTray {
    /// Ask the host to open the menu when the tray icon is activated.
    ///
    /// Screenshaver does not have a primary tray-icon action, so opening the
    /// menu is the most useful response to either left- or right-clicking it.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(
        &self,
    ) -> String {
        APPLICATION_ID.into()
    }

    fn title(
        &self,
    ) -> String {
        APPLICATION_NAME.into()
    }

    fn category(
        &self,
    ) -> Category {
        Category::ApplicationStatus
    }

    fn status(
        &self,
    ) -> Status {
        Status::Active
    }

    fn icon_name(
        &self,
    ) -> String {
        ICON_NAME.into()
    }

    fn tool_tip(
        &self,
    ) -> ToolTip {
        ToolTip {
            icon_name:
                ICON_NAME.into(),
            title:
                APPLICATION_NAME.into(),
            description:
                TOOLTIP_DESCRIPTION.into(),
            ..Default::default()
        }
    }

    fn menu(
        &self,
    ) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label:
                    format!(
                        "Screensaver: {}",
                        enabled_status(
                            self.status
                                .screensaver_enabled
                        ),
                    ),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label:
                    "Wallpaper:".into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label:
                    self.status
                        .wallpaper
                        .wallpaper_label(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label:
                    "Edit".into(),
                icon_name:
                    "document-edit".into(),
                enabled:
                    self.status.wallpaper.active_wallpaper().is_some(),
                activate:
                    Box::new(
                        |tray: &mut Self| {
                            tray.send_command(
                                TrayCommand::EditWallpaper
                            );
                        }
                    ),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label:
                    "Restart".into(),
                icon_name:
                    "view-refresh".into(),
                activate:
                    Box::new(
                        |tray: &mut Self| {
                            tray.send_command(
                                TrayCommand::Restart
                            );
                        }
                    ),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label:
                    "Stop".into(),
                icon_name:
                    "application-exit".into(),
                activate:
                    Box::new(
                        |tray: &mut Self| {
                            tray.send_command(
                                TrayCommand::Stop
                            );
                        }
                    ),
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
    command_sender:
        Sender<TrayCommand>,
    status:
        TrayStatus,
) -> Result<TrayHandle, ksni::Error> {
    let wallpaper_status =
        status.wallpaper.clone();

    let handle =
        ScreenshaverTray::new(
            command_sender,
            status,
        )
        .spawn()?;

    wallpaper_status.attach_handle(
        handle.clone()
    );

    Ok(handle)
}

fn enabled_status(
    enabled: bool,
) -> &'static str {
    if enabled {
        "Enabled"
    } else {
        "Disabled"
    }
}

