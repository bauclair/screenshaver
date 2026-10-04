//! LXDE/X11 XEmbed tray prototype: icon docking only.
//! Menu commands are deliberately reserved for stage 2.
use std::ffi::CString;
use std::os::raw::{c_long, c_uchar};
use std::path::Path;
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
pub fn start(logfile: &Path) -> Result<X11TrayHandle, String> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    let log = logfile.to_path_buf();
    let thread = thread::Builder::new()
        .name("screenshaver-xembed-tray".into())
        .spawn(move || {
            let result = unsafe { run_tray(&log, &shutdown_rx, &ready_tx) };
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
) -> Result<(), String> {
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
    if display.is_null() { return Err("Cannot open X11 display".into()); }
    let result = unsafe { run_with_display(display, logfile, shutdown, ready) };
    unsafe { xlib::XCloseDisplay(display); }
    result
}

unsafe fn run_with_display(
    display: *mut xlib::Display,
    logfile: &Path,
    shutdown: &Receiver<()>,
    ready: &mpsc::Sender<Result<(), String>>,
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
    let icon = unsafe { xlib::XCreateSimpleWindow(display, root, 0, 0, 32, 32, 0, 0, 0) };
    if icon == 0 { return Err("XCreateSimpleWindow failed".into()); }
    // XEMBED_MAPPED = 1; version = 0. The property format is 32 bits,
    // but Xlib expects native-long storage for format-32 properties.
    let info: [c_long; 2] = [0, 1];
    unsafe {
        xlib::XChangeProperty(display, icon, embed, embed, 32, xlib::PropModeReplace,
            info.as_ptr() as *const c_uchar, 2);
        xlib::XSelectInput(display, icon, xlib::ExposureMask | xlib::StructureNotifyMask);
    }
    let decoded = image::load_from_memory(ARTWORK)
        .map_err(|e| format!("Unable to decode embedded tray artwork: {e}"))?
        .to_rgba8();
    let gc = unsafe { xlib::XCreateGC(display, icon, 0, std::ptr::null_mut()) };
    // Request docking before mapping; the tray manager reparents the icon.
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
        xlib::XSendEvent(display, owner, xlib::False, xlib::NoEventMask, &mut event);
        xlib::XMapWindow(display, icon);
        draw_icon(display, icon, gc, &decoded);
        xlib::XFlush(display);
    }
    crate::logger::information(logfile,
        &format!("[TRAY/X11] Sent XEmbed dock request: owner=0x{owner:X}, icon=0x{icon:X}"));
    let _ = ready.send(Ok(()));
    loop {
        if shutdown.try_recv().is_ok() { break; }
        while unsafe { xlib::XPending(display) } > 0 {
            let mut next: xlib::XEvent = unsafe { std::mem::zeroed() };
            unsafe { xlib::XNextEvent(display, &mut next); }
            if next.get_type() == xlib::Expose {
                unsafe { draw_icon(display, icon, gc, &decoded); }
            } else if next.get_type() == xlib::DestroyNotify {
                crate::logger::warning(logfile, "[TRAY/X11] Tray icon window destroyed");
                unsafe { xlib::XFreeGC(display, gc); }
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(40));
    }
    unsafe { xlib::XFreeGC(display, gc); xlib::XDestroyWindow(display, icon); }
    Ok(())
}

unsafe fn draw_icon(
    display: *mut xlib::Display,
    window: xlib::Window,
    gc: xlib::GC,
    pixels: &image::RgbaImage,
) {
    // For this first-stage prototype, paint directly into the XEmbed window.
    // Convert RGB to the server's TrueColor masks rather than assuming RGB/BGR order.
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
    for (x, y, pixel) in pixels.enumerate_pixels() {
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
    unsafe { xlib::XFlush(display); }
}
