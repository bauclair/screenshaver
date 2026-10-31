//! Temporary KDE Plasma 6 native-wallpaper transport test.
//! Rust-only producer compatible with the Stage 2D C++/Qt FrameItem.
//! No production wallpaper renderer or database is used.

use std::fs::{self, OpenOptions};

use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;
const SLOTS: usize = 3;
const FRAME: usize = WIDTH * HEIGHT * 4;
const HEADER: usize = 64;
const TOTAL: usize = HEADER + SLOTS * FRAME;
const FPS: u64 = 30;

struct Lock<'a> { fd: &'a std::fs::File }
impl<'a> Lock<'a> {
    fn exclusive(fd: &'a std::fs::File, nonblocking: bool) -> Result<Self, String> {
        let flags = libc::LOCK_EX | if nonblocking { libc::LOCK_NB } else { 0 };
        if unsafe { libc::flock(fd.as_raw_fd(), flags) } != 0 {
            return Err(format!("flock failed: {}", std::io::Error::last_os_error()));
        }
        Ok(Self { fd })
    }
}
impl Drop for Lock<'_> {
    fn drop(&mut self) { unsafe { libc::flock(self.fd.as_raw_fd(), libc::LOCK_UN); } }
}

struct Mapping { ptr: *mut u8 }
impl Mapping {
    fn new(fd: &std::fs::File) -> Result<Self, String> {
        let ptr = unsafe {
            libc::mmap(std::ptr::null_mut(), TOTAL, libc::PROT_READ | libc::PROT_WRITE,
                       libc::MAP_SHARED, fd.as_raw_fd(), 0)
        };
        if ptr == libc::MAP_FAILED { return Err(format!("mmap failed: {}", std::io::Error::last_os_error())); }
        Ok(Self { ptr: ptr.cast() })
    }
    fn bytes(&mut self) -> &mut [u8] { unsafe { std::slice::from_raw_parts_mut(self.ptr, TOTAL) } }
}
impl Drop for Mapping {
    fn drop(&mut self) { unsafe { libc::munmap(self.ptr.cast(), TOTAL); } }
}

pub fn run() -> Result<(), String> {
    if unsafe { libc::geteuid() } == 0 { return Err("Do not run this test as root".into()); }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .ok_or("XDG_RUNTIME_DIR must identify an existing directory")?;
    let uid = unsafe { libc::getuid() };
    let base = runtime.join(format!("screenshaver-native-shm-{uid}"));
    let lock_path = base.with_extension("lock");
    let bin_path = base.with_extension("bin");
    let owner_path = base.with_extension("owner");

    // Persistent inode, never unlinked: concurrent producers cannot acquire
    // separate locks by racing a pathname replacement.
    let owner = OpenOptions::new().read(true).write(true).create(true)
        .mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&owner_path).map_err(|e| format!("owner lock: {e}"))?;
    let _owner_guard = Lock::exclusive(&owner, true)
        .map_err(|_| "Another KDE wallpaper test producer owns this transport".to_string())?;
    let lock = OpenOptions::new().read(true).write(true).create(true)
        .mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&lock_path).map_err(|e| format!("frame lock: {e}"))?;
    let file = OpenOptions::new().read(true).write(true).create(true)
        .mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&bin_path).map_err(|e| format!("frame file: {e}"))?;
    fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("frame permissions: {e}"))?;
    file.set_len(TOTAL as u64).map_err(|e| format!("frame sizing: {e}"))?;
    let mut mapping = Mapping::new(&file)?;
    {
        let _guard = Lock::exclusive(&lock, false)?;
        let data = mapping.bytes();
        data.fill(0);
        data[0..8].copy_from_slice(b"SSNAT001");
        for (offset, value) in [(8, WIDTH as u32), (12, HEIGHT as u32), (16, 4), (20, SLOTS as u32)] {
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let stop = Arc::clone(&running);
    ctrlc::set_handler(move || stop.store(false, Ordering::Relaxed))
        .map_err(|e| format!("signal handler: {e}"))?;
    println!("[KDE TEST] Rust native producer: {WIDTH}x{HEIGHT}, RGBA8888, {FPS} FPS");
    println!("[KDE TEST] Transport: {}", bin_path.display());
    println!("[KDE TEST] Start/enable the existing Stage 2D Plasma wallpaper; Ctrl+C stops the producer.");

    let started = Instant::now();
    let mut deadline = Instant::now();
    let mut sequence = 0_u64;
    let mut row = vec![0_u8; WIDTH * 4];
    while running.load(Ordering::Relaxed) {
        sequence += 1;
        let phase = (sequence & 255) as usize;
        for x in 0..WIDTH {
            let at = x * 4;
            row[at] = ((x + phase) & 255) as u8;
            row[at + 1] = ((phase * 3) & 255) as u8;
            row[at + 2] = ((255_isize - x as isize / 3).rem_euclid(256)) as u8;
            row[at + 3] = 255;
        }
        {
            let _guard = Lock::exclusive(&lock, false)?;
            let data = mapping.bytes();
            let offset = HEADER + (sequence as usize % SLOTS) * FRAME;
            for y in 0..HEIGHT {
                let start = offset + y * row.len();
                data[start..start + row.len()].copy_from_slice(&row);
            }
            // Commit sequence after all pixel writes, while lock is held.
            data[24..32].copy_from_slice(&sequence.to_le_bytes());
        }
        if sequence % (FPS * 5) == 0 {
            println!("[KDE TEST] published={sequence}, average FPS={:.1}", sequence as f64 / started.elapsed().as_secs_f64());
        }
        deadline += Duration::from_nanos(1_000_000_000 / FPS);
        let now = Instant::now();
        if deadline > now { thread::sleep(deadline - now); }
        else { deadline = now; }
    }
    // Drop mapping before removing the pathname. The C++ consumer detects its
    // disappearance and reconnects on the next producer run.
    drop(mapping);
    drop(file);
    fs::remove_file(&bin_path).map_err(|e| format!("remove frame file: {e}"))?;
    println!("[KDE TEST] stopped after {sequence} frames.");
    Ok(())
}
