//! Isolated Screenshaver KDE wallpaper frame producer.
//!
//! Staging executable: deliberately does not modify Plasma wallpaper ownership.
//! Renders the production wallpaper mode (C ABI mode=1) into a private SDL/OpenGL
//! context, then publishes opaque RGBA frames in a versioned double-buffered map.
//! The existing Rust FrameRenderEngine remains the only shader implementation.
use std::{env, error::Error, ffi::{c_char, c_void, CStr}, fs::{self, OpenOptions}, io::{self, BufRead}, os::unix::fs::MetadataExt, path::PathBuf, sync::{atomic::{AtomicBool, AtomicU64, Ordering}, mpsc, Arc}, thread, time::{Duration, Instant}};
use libloading::{Library, Symbol};
use memmap2::MmapMut;
use sdl2::video::GLProfile;
use fs2::FileExt;

const MAGIC: &[u8; 8] = b"SSPLFR01";
const HEADER: usize = 64;
const MAX_BYTES: usize = 256 * 1024 * 1024;

type Create = unsafe extern "C" fn(Option<unsafe extern "C" fn(*const c_char) -> *const c_void>, i32, i32, u32) -> *mut c_void;
type Render = unsafe extern "C" fn(*mut c_void, i32, i32) -> bool;
type Destroy = unsafe extern "C" fn(*mut c_void);
type LastError = unsafe extern "C" fn() -> *const c_char;
type Reload = unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn(*const c_char) -> *const c_void>, i32, i32) -> bool;

unsafe extern "C" fn get_gl_proc(name: *const c_char) -> *const c_void {
    // SDL owns the OpenGL context and its procedure resolver.
    unsafe { sdl2::sys::SDL_GL_GetProcAddress(name) as *const c_void }
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn usage() -> &'static str {
    "Usage: screenshaver-plasma-producer <libscreenshaver.so> <output.frames> <width> <height> [fps]"
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 || args.len() > 6 { return Err(usage().into()); }
    let library_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);
    let width: u32 = args[3].parse()?;
    let height: u32 = args[4].parse()?;
    let fps: u32 = args.get(5).map(|v| v.parse()).transpose()?.unwrap_or(30);
    if width == 0 || height == 0 || width > 16384 || height > 16384 || !(1..=240).contains(&fps) {
        return Err("Invalid dimensions or FPS".into());
    }
    let stride = width.checked_mul(4).ok_or("Stride overflow")? as usize;
    let frame_size = stride.checked_mul(height as usize).ok_or("Frame overflow")?;
    let total = HEADER.checked_add(frame_size.checked_mul(2).ok_or("Buffer overflow")?).ok_or("Size overflow")?;
    if total > MAX_BYTES { return Err("Frame transport exceeds 256 MiB".into()); }
    if output_path.exists() { return Err("Output frame file already exists; refusing to replace it".into()); }
    let file = OpenOptions::new().write(true).read(true).create_new(true).open(&output_path)?;
    // Retire only the inode created by this producer. Never unlink a file
    // replaced by another process during shutdown or error unwinding.
    struct OutputGuard { path: PathBuf, dev: u64, ino: u64 }
    impl Drop for OutputGuard {
        fn drop(&mut self) {
            if let Ok(metadata) = fs::symlink_metadata(&self.path) {
                if metadata.is_file() && metadata.dev() == self.dev && metadata.ino() == self.ino {
                    if let Err(error) = fs::remove_file(&self.path) {
                        eprintln!("[PLASMA PRODUCER] Cannot retire frame file: {error}");
                    }
                }
            }
        }
    }
    let metadata = file.metadata()?;
    let _output_guard = OutputGuard {
        path: output_path.clone(), dev: metadata.dev(), ino: metadata.ino(),
    };
    file.set_len(total as u64)?;
    let mut map = unsafe { MmapMut::map_mut(&file)? };
    map[..HEADER].fill(0);
    map[..8].copy_from_slice(MAGIC);
    write_u32(&mut map, 8, 1); // protocol version
    write_u32(&mut map, 12, HEADER as u32);
    write_u32(&mut map, 24, width);
    write_u32(&mut map, 28, height);
    write_u32(&mut map, 32, stride as u32);
    write_u32(&mut map, 36, frame_size as u32);
    // Byte offset 16: aligned atomic u64 publication counter. Zero = no frame.
    // A consumer reads it with Acquire and uses (sequence - 1) % 2 for slot.
    let seq_ptr = unsafe { map.as_mut_ptr().add(16) as *mut AtomicU64 };
    if (seq_ptr as usize) % std::mem::align_of::<AtomicU64>() != 0 {
        return Err("Unaligned publication counter".into());
    }
    let sdl = sdl2::init().map_err(io::Error::other)?;
    let video = sdl.video().map_err(io::Error::other)?;
    let gl_attr = video.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);
    let window = video.window("Screenshaver isolated wallpaper producer", width, height)
        .opengl().hidden().build()?;
    let context = window.gl_create_context().map_err(io::Error::other)?;
    window.gl_make_current(&context).map_err(io::Error::other)?;
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    unsafe { gl::PixelStorei(gl::PACK_ALIGNMENT, 1); }
    // Keep the library mapped until its renderer has been destroyed.
    let library = unsafe { Library::new(&library_path)? };
    let create: Symbol<Create> = unsafe { library.get(b"screenshaver_kde_gl_create_for_mode\0")? };
    let render: Symbol<Render> = unsafe { library.get(b"screenshaver_kde_gl_render\0")? };
    let destroy: Symbol<Destroy> = unsafe { library.get(b"screenshaver_kde_gl_destroy\0")? };
    let last_error: Symbol<LastError> = unsafe { library.get(b"screenshaver_kde_gl_last_error\0")? };
    let reload: Symbol<Reload> = unsafe { library.get(b"screenshaver_kde_gl_reload_wallpaper\0")? };
    let renderer = unsafe { create(Some(get_gl_proc), width as i32, height as i32, 1) };
    if renderer.is_null() {
        let ptr = unsafe { last_error() };
        let message = if ptr.is_null() { "Unknown renderer error".into() }
            else { unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned() };
        return Err(format!("Cannot initialize wallpaper renderer: {message}").into());
    }
    struct Guard { renderer: *mut c_void, destroy: Destroy }
    impl Drop for Guard { fn drop(&mut self) { unsafe { (self.destroy)(self.renderer) }; } }
    let guard = Guard { renderer, destroy: *destroy };
    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    ctrlc::set_handler(move || stop_signal.store(true, Ordering::Relaxed))?;
    // Commands arrive over a dedicated stdin reader. This thread never touches
    // the GL context; only the render thread changes rendering state.
    // EOF is treated as STOP: a detached producer must not render indefinitely.
    let (command_tx, command_rx) = mpsc::channel::<String>();
    thread::Builder::new().name("plasma-producer-control".into()).spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(line) => { if command_tx.send(line).is_err() { return; } }
                Err(_) => break,
            }
        }
        let _ = command_tx.send("STOP".into());
    })?;
    let mut paused = false;
    let mut scratch = vec![0u8; frame_size];
    let frame_period = Duration::from_secs_f64(1.0 / fps as f64);
    let mut next_frame = Instant::now();
    let mut seq = 0u64;
    eprintln!("[PLASMA PRODUCER] rendering {width}x{height} at up to {fps} fps into {}", output_path.display());
    while !stop.load(Ordering::Relaxed) {
        // Acknowledgments are emitted only after the render thread has
        // applied a command. PAUSED means no subsequent frame will be
        // published until RESUME is processed. The frame sequence is stable.
        while let Ok(command) = command_rx.try_recv() {
            match command.trim() {
                "PAUSE" => {
                    paused = true;
                    eprintln!("[PLASMA CONTROL] PAUSED sequence={seq}");
                }
                "RESUME" => {
                    paused = false;
                    next_frame = Instant::now();
                    eprintln!("[PLASMA CONTROL] RESUMED sequence={seq}");
                }
                "RELOAD" => {
                    if unsafe { reload(guard.renderer, Some(get_gl_proc), width as i32, height as i32) } {
                        eprintln!("[PLASMA CONTROL] RELOADED sequence={seq}");
                    } else {
                        let ptr = unsafe { last_error() };
                        let message = if ptr.is_null() { "Unknown reload error".into() }
                            else { unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned() };
                        eprintln!("[PLASMA CONTROL] RELOAD_FAILED sequence={seq} reason={message:?}");
                    }
                }
                "STOP" => {
                    stop.store(true, Ordering::Relaxed);
                    eprintln!("[PLASMA CONTROL] STOPPING sequence={seq}");
                }
                other => eprintln!("[PLASMA CONTROL] UNSUPPORTED command={other:?}"),
            }
        }
        if stop.load(Ordering::Relaxed) { break; }
        if paused {
            match command_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(command) => {
                    // Feed the command back through the same command-processing
                    // path without busy-spinning or rendering while paused.
                    match command.trim() {
                        "RESUME" => {
                            paused = false;
                            next_frame = Instant::now();
                            eprintln!("[PLASMA CONTROL] RESUMED sequence={seq}");
                        }
                        "RELOAD" => {
                            if unsafe { reload(guard.renderer, Some(get_gl_proc), width as i32, height as i32) } {
                                eprintln!("[PLASMA CONTROL] RELOADED sequence={seq}");
                            } else {
                                let ptr = unsafe { last_error() };
                                let message = if ptr.is_null() { "Unknown reload error".into() }
                                    else { unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned() };
                                eprintln!("[PLASMA CONTROL] RELOAD_FAILED sequence={seq} reason={message:?}");
                            }
                        }
                        "STOP" => {
                            stop.store(true, Ordering::Relaxed);
                            eprintln!("[PLASMA CONTROL] STOPPING sequence={seq}");
                        }
                        "PAUSE" => eprintln!("[PLASMA CONTROL] PAUSED sequence={seq}"),
                        other => eprintln!("[PLASMA CONTROL] UNSUPPORTED command={other:?}"),
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => stop.store(true, Ordering::Relaxed),
                Err(mpsc::RecvTimeoutError::Timeout) => {},
            }
            continue;
        }
        // The producer never modifies Plasma wallpaper selection.
        if !unsafe { render(guard.renderer, width as i32, height as i32) } {
            let ptr = unsafe { last_error() };
            let message = if ptr.is_null() { "Unknown render error".into() }
                else { unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned() };
            return Err(format!("Wallpaper frame render failed: {message}").into());
        }
        unsafe {
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, 0);
            gl::ReadBuffer(gl::BACK);
            gl::ReadPixels(0, 0, width as i32, height as i32, gl::RGBA, gl::UNSIGNED_BYTE, scratch.as_mut_ptr().cast());
        }
        // OpenGL rows are bottom-up; the eventual presenter must flip vertically.
        // Advisory file lock prevents the Qt reader from observing a partially
        // copied slot or an overwrite during its frame snapshot.
        file.lock_exclusive()?;
        let slot = (seq % 2) as usize;
        let start = HEADER + slot * frame_size;
        map[start..start + frame_size].copy_from_slice(&scratch);
        seq = seq.checked_add(1).ok_or("Sequence overflow")?;
        unsafe { (*seq_ptr).store(seq, Ordering::Release) };
        FileExt::unlock(&file)?;
        next_frame += frame_period;
        let now = Instant::now();
        if next_frame > now { thread::sleep(next_frame - now); }
        else { next_frame = now; }
    }
    eprintln!("[PLASMA PRODUCER] stopping after {seq} frames");
    drop(guard); // destroy GL renderer before SDL context teardown
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("[PLASMA PRODUCER] ERROR: {error}");
        std::process::exit(1);
    }
}
