//! X11/XEmbed system-tray backend for notification areas such as LXPanel.
//!
//! The icon must remain unmapped until the tray manager accepts the dock request
//! and maps the embedded window. Mapping it before docking can cause LXPanel to
//! detach the icon immediately after the XEmbed handshake.
use std::ffi::CString;
use std::os::raw::{c_long, c_uchar};
use std::path::Path;
use std::sync::mpsc::Sender;
use crate::tray_icon::{TrayCommand, TrayStatus};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use x11::xlib;

const ARTWORK: &[u8] = include_bytes!("../assets/icons/hicolor/32x32/apps/screenshaver.png");

pub struct X11TrayHandle {
    shutdown: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for X11TrayHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Start a separate X connection so tray events do not block the renderer.
/// The startup handshake reports a missing notification-area owner immediately.
pub fn start(
    logfile: &Path,
    command_sender: Sender<TrayCommand>,
    status: TrayStatus,
) -> Result<X11TrayHandle, String> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    let log = logfile.to_path_buf();
    let thread = thread::Builder::new()
        .name("screenshaver-xembed-tray".into())
        .spawn(move || {
            let result = unsafe { run_tray(&log, &shutdown_rx, &ready_tx, &command_sender, &status) };
            if let Err(error) = result {
                let _ = ready_tx.send(Err(error.clone()));
                crate::logger::warning(&log, &format!("[TRAY/X11] {}", error));
            }
        })
        .map_err(|e| format!("Unable to start XEmbed thread: {e}"))?;
    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok(X11TrayHandle { shutdown: shutdown_tx, thread: Some(thread) }),
        Ok(Err(error)) => { let _ = thread.join(); Err(error) }
        Err(error) => { let _ = shutdown_tx.send(()); let _ = thread.join(); Err(format!("XEmbed startup handshake failed: {error}")) }
    }
}

unsafe fn run_tray(
    logfile: &Path,
    shutdown: &Receiver<()>,
    ready: &mpsc::Sender<Result<(), String>>,
    command_sender: &Sender<TrayCommand>,
    status: &TrayStatus,
) -> Result<(), String> {
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
    if display.is_null() { return Err("Cannot open X11 display".into()); }
    let result = unsafe { run_with_display(display, logfile, shutdown, ready, command_sender, status) };
    unsafe { xlib::XCloseDisplay(display); }
    result
}

unsafe fn run_with_display(
    display: *mut xlib::Display,
    logfile: &Path,
    shutdown: &Receiver<()>,
    ready: &mpsc::Sender<Result<(), String>>,
    command_sender: &Sender<TrayCommand>,
    status: &TrayStatus,
) -> Result<(), String> {
    let screen = unsafe { xlib::XDefaultScreen(display) };
    let root = unsafe { xlib::XRootWindow(display, screen) };
    let selection = CString::new(format!("_NET_SYSTEM_TRAY_S{screen}")).unwrap();
    let selection_atom = unsafe { xlib::XInternAtom(display, selection.as_ptr(), xlib::False) };
    let owner = unsafe { xlib::XGetSelectionOwner(display, selection_atom) };
    if owner == 0 { return Err("No XEmbed notification-area manager owns the system-tray selection".into()); }
    let embed_atom = CString::new("_XEMBED_INFO").unwrap();
    let opcode_atom = CString::new("_NET_SYSTEM_TRAY_OPCODE").unwrap();
    let embed = unsafe { xlib::XInternAtom(display, embed_atom.as_ptr(), xlib::False) };
    let opcode = unsafe { xlib::XInternAtom(display, opcode_atom.as_ptr(), xlib::False) };
    let xembed_name = CString::new("_XEMBED").unwrap();
    let xembed_message = unsafe { xlib::XInternAtom(display, xembed_name.as_ptr(), xlib::False) };
    let icon = unsafe { xlib::XCreateSimpleWindow(display, root, 0, 0, 32, 32, 0, 0, 0) };
    if icon == 0 { return Err("XCreateSimpleWindow failed".into()); }
    // XEMBED_MAPPED = 1; version = 0. Property type must be CARDINAL.
    // The property format is 32 bits,
    // but Xlib expects native-long storage for format-32 properties.
    let info: [c_long; 2] = [0, 1];
    unsafe {
        xlib::XChangeProperty(display, icon, embed, xlib::XA_CARDINAL, 32, xlib::PropModeReplace,
            info.as_ptr() as *const c_uchar, 2);
        xlib::XSelectInput(display, icon, xlib::ExposureMask | xlib::StructureNotifyMask | xlib::ButtonPressMask | xlib::ButtonReleaseMask);
    }
    let decoded = image::load_from_memory(ARTWORK)
        .map_err(|e| format!("Unable to decode embedded tray artwork: {e}"))?
        .to_rgba8();
    let gc = unsafe { xlib::XCreateGC(display, icon, 0, std::ptr::null_mut()) };
    // Do not map the icon here. The XEmbed tray manager owns initial mapping.
    let mut message: xlib::XClientMessageEvent = unsafe { std::mem::zeroed() };
    message.type_ = xlib::ClientMessage;
    message.window = owner;
    message.message_type = opcode;
    message.format = 32;
    message.data.set_long(0, xlib::CurrentTime as c_long);
    message.data.set_long(1, 0); // SYSTEM_TRAY_REQUEST_DOCK
    message.data.set_long(2, icon as c_long);
    let mut event: xlib::XEvent = unsafe { std::mem::zeroed() };
    unsafe {
        std::ptr::write(
            (&mut event as *mut xlib::XEvent).cast::<xlib::XClientMessageEvent>(),
            message,
        );
    }
    unsafe {
        let sent = xlib::XSendEvent(display, owner, xlib::False, xlib::NoEventMask, &mut event);
        if sent == 0 {
            crate::logger::warning(logfile, "[TRAY/X11] XEmbed dock request was not delivered");
        }
        draw_icon(display, icon, gc, &decoded);
        xlib::XFlush(display);
    }
    crate::logger::information(logfile, "[TRAY/X11] XEmbed tray backend started");
    let _ = ready.send(Ok(()));
    // Override-redirect popup is independent of the embedded icon window.
    let menu = unsafe { xlib::XCreateSimpleWindow(display, root, 0, 0, 220, 180, 1,
        xlib::XBlackPixel(display, screen), xlib::XWhitePixel(display, screen)) };
    unsafe {
        let mut attrs: xlib::XSetWindowAttributes = std::mem::zeroed();
        attrs.override_redirect = xlib::True;
        xlib::XChangeWindowAttributes(display, menu, xlib::CWOverrideRedirect, &mut attrs);
        xlib::XSelectInput(display, menu, xlib::ExposureMask | xlib::ButtonPressMask);
    }
    let menu_gc = unsafe { xlib::XCreateGC(display, menu, 0, std::ptr::null_mut()) };
    let mut menu_open = false;
    loop {
        if shutdown.try_recv().is_ok() {
            crate::logger::information(logfile, "[TRAY/X11] Shutdown requested by tray handle");
            break;
        }
        while unsafe { xlib::XPending(display) } > 0 {
            let mut next: xlib::XEvent = unsafe { std::mem::zeroed() };
            unsafe { xlib::XNextEvent(display, &mut next); }
            match next.get_type() {
                xlib::Expose => {
                    let e = unsafe { &*((&next as *const xlib::XEvent).cast::<xlib::XExposeEvent>()) };
                    if e.window == icon { unsafe { draw_icon(display, icon, gc, &decoded); } }
                    else if e.window == menu { unsafe { paint_menu(display, menu, menu_gc, screen, status); } }
                },
                xlib::ButtonPress => {
                    let e = unsafe { &*((&next as *const xlib::XEvent).cast::<xlib::XButtonEvent>()) };
                    if e.window == icon && (e.button == 1 || e.button == 3) {
                        if menu_open {
                            unsafe { close_menu(display, menu); }
                            menu_open = false;
                        } else {
                            let screen_width = unsafe { xlib::XDisplayWidth(display, screen) };
                            let screen_height = unsafe { xlib::XDisplayHeight(display, screen) };
                            let x = e.x_root.clamp(0, (screen_width - 222).max(0));
                            let y = (e.y_root - 180).clamp(0, (screen_height - 182).max(0));
                            unsafe {
                                xlib::XMoveWindow(display, menu, x, y);
                                xlib::XMapRaised(display, menu);
                                paint_menu(display, menu, menu_gc, screen, status);
                                let grab = xlib::XGrabPointer(display, menu, xlib::False,
                                    xlib::ButtonPressMask as u32, xlib::GrabModeAsync,
                                    xlib::GrabModeAsync, 0, 0, xlib::CurrentTime);
                                if grab != xlib::GrabSuccess {
                                    crate::logger::warning(logfile, &format!("[TRAY/X11] Menu pointer grab failed: {grab}"));
                                }
                            }
                            menu_open = true;
                        }
                    } else if menu_open && e.window == menu {
                        // With the pointer grabbed, outside clicks have coordinates outside the popup.
                        let chosen = if e.x >= 0 && e.x < 220 {
                            match e.y {
                                90..=119 => Some(TrayCommand::Edit),
                                120..=149 => Some(TrayCommand::Restart),
                                150..=179 => Some(TrayCommand::Stop),
                                _ => None,
                            }
                        } else { None };
                        unsafe { close_menu(display, menu); }
                        menu_open = false;
                        if let Some(command) = chosen {
                            crate::logger::information(logfile, &format!("[TRAY/X11] Menu command: {command:?}"));
                            let _ = command_sender.send(command);
                        }
                    }
                },
                xlib::UnmapNotify => {
                    let e = unsafe { &*((&next as *const xlib::XEvent).cast::<xlib::XUnmapEvent>()) };
                    crate::logger::warning(logfile, &format!("[TRAY/X11] UnmapNotify: window=0x{:X}, icon={}, event=0x{:X}", e.window, e.window == icon, e.event));
                }
                xlib::ClientMessage => {
                    let event = unsafe {
                        &*((&next as *const xlib::XEvent).cast::<xlib::XClientMessageEvent>())
                    };
                    if event.message_type == xembed_message
                        && event.format == 32
                        && event.data.get_long(1) == 0
                    {
                        crate::logger::information(
                            logfile,
                            "[TRAY/X11] XEmbed dock request accepted",
                        );
                    }
                }
                xlib::DestroyNotify => {
                let e = unsafe { &*((&next as *const xlib::XEvent).cast::<xlib::XDestroyWindowEvent>()) };
                if e.window != icon { continue; }
                crate::logger::warning(logfile, "[TRAY/X11] Tray icon window destroyed");
                unsafe { xlib::XFreeGC(display, menu_gc); xlib::XDestroyWindow(display, menu); xlib::XFreeGC(display, gc); }
                return Ok(());
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(40));
    }
    unsafe { if menu_open { close_menu(display, menu); } xlib::XFreeGC(display, menu_gc); xlib::XDestroyWindow(display, menu); xlib::XFreeGC(display, gc); xlib::XDestroyWindow(display, icon); }
    Ok(())
}

unsafe fn draw_icon(
    display: *mut xlib::Display,
    window: xlib::Window,
    gc: xlib::GC,
    pixels: &image::RgbaImage,
) {
    // Paint directly into the XEmbed window using the server's TrueColor masks
    // rather than assuming a particular RGB/BGR byte order.
    let screen = unsafe { xlib::XDefaultScreen(display) };
    let visual = unsafe { xlib::XDefaultVisual(display, screen) };
    let (red_mask, green_mask, blue_mask) = unsafe {
        ((*visual).red_mask, (*visual).green_mask, (*visual).blue_mask)
    };
    let component = |value: u8, mask: u64| -> u64 {
        if mask == 0 { return 0; }
        let shift = mask.trailing_zeros();
        let maximum = mask >> shift;
        (((value as u64 * maximum + 127) / 255) << shift) & mask
    };
    let mut attrs: xlib::XWindowAttributes = unsafe { std::mem::zeroed() };
    if unsafe { xlib::XGetWindowAttributes(display, window, &mut attrs) } == 0 { return; }
    let width = attrs.width.max(1) as u32;
    let height = attrs.height.max(1) as u32;
    for y in 0..height {
      for x in 0..width {
        let source_x = x * pixels.width() / width;
        let source_y = y * pixels.height() / height;
        let pixel = pixels.get_pixel(source_x, source_y);
        let alpha = pixel[3] as u16;
        let r = (pixel[0] as u16 * alpha / 255) as u8;
        let g = (pixel[1] as u16 * alpha / 255) as u8;
        let b = (pixel[2] as u16 * alpha / 255) as u8;
        let value = component(r, red_mask as u64)
            | component(g, green_mask as u64)
            | component(b, blue_mask as u64);
        unsafe {
            xlib::XSetForeground(display, gc, value as _);
            xlib::XFillRectangle(display, window, gc, x as i32, y as i32, 1, 1);
        }
      }
    }
    unsafe { xlib::XFlush(display); }
}

unsafe fn close_menu(display: *mut xlib::Display, menu: xlib::Window) {
    unsafe { xlib::XUngrabPointer(display, xlib::CurrentTime); xlib::XUnmapWindow(display, menu); xlib::XFlush(display); }
}

unsafe fn paint_menu(
    display: *mut xlib::Display,
    menu: xlib::Window,
    gc: xlib::GC,
    screen: i32,
    status: &TrayStatus,
) {
    let screensaver_status = if status.screensaver_enabled {
        crate::manage_localization::runtime_text("tray.status.enabled")
    } else {
        crate::manage_localization::runtime_text("tray.status.disabled")
    };
    let screensaver_label = crate::manage_localization::runtime_text_with_params(
        "tray.menu.screensaver_status",
        &[("status", &screensaver_status)],
    );
    let wallpaper_heading = crate::manage_localization::runtime_text("tray.menu.wallpaper");
    let wallpaper_label = status.wallpaper.wallpaper_label();
    let edit_label = crate::manage_localization::runtime_text("tray.menu.edit");
    let restart_label = crate::manage_localization::runtime_text("tray.menu.restart");
    let stop_label = crate::manage_localization::runtime_text("tray.menu.stop");
    let labels = [
        screensaver_label,
        wallpaper_heading,
        wallpaper_label,
        edit_label,
        restart_label,
        stop_label,
    ];

    unsafe {
        xlib::XSetForeground(display, gc, xlib::XWhitePixel(display, screen));
        xlib::XFillRectangle(display, menu, gc, 0, 0, 220, 180);
        xlib::XSetForeground(display, gc, xlib::XBlackPixel(display, screen));
        for (i, label) in labels.iter().enumerate() {
            xlib::XDrawString(
                display,
                menu,
                gc,
                12,
                20 + i as i32 * 30,
                label.as_ptr().cast(),
                label.len() as i32,
            );
            if i < labels.len() - 1 {
                xlib::XDrawLine(
                    display,
                    menu,
                    gc,
                    4,
                    29 + i as i32 * 30,
                    215,
                    29 + i as i32 * 30,
                );
            }
        }
        xlib::XFlush(display);
    }
}
