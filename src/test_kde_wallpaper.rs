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

pub fn run(shader_path: Option<&str>) -> Result<(), String> {
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

    let mut renderer = match shader_path {
        Some(path) => Some(OffscreenRenderer::new(path)?),
        None => None,
    };
    if let Some(path) = shader_path {
        println!("[KDE TEST] OpenGL shader: {path}");
    }
    let mut timings = Timings::default();
    let started = Instant::now();
    let mut deadline = Instant::now();
    let mut sequence = 0_u64;
    let mut row = vec![0_u8; WIDTH * 4];
    while running.load(Ordering::Relaxed) {
        sequence += 1;
        let mut render_elapsed = Duration::ZERO;
        let mut readback_elapsed = Duration::ZERO;
        let mut pixels = Vec::new();
        if let Some(renderer) = renderer.as_mut() {
            let (frame, render, readback) = renderer.render(started.elapsed().as_secs_f32(), sequence)?;
            pixels = frame;
            render_elapsed = render;
            readback_elapsed = readback;
        } else {
            let phase = (sequence & 255) as usize;
            for x in 0..WIDTH {
                let at = x * 4;
                row[at] = ((x + phase) & 255) as u8;
                row[at + 1] = ((phase * 3) & 255) as u8;
                row[at + 2] = ((255_isize - x as isize / 3).rem_euclid(256)) as u8;
                row[at + 3] = 255;
            }
        }
        let publish_started = Instant::now();
        {
            let _guard = Lock::exclusive(&lock, false)?;
            let data = mapping.bytes();
            let offset = HEADER + (sequence as usize % SLOTS) * FRAME;
            if renderer.is_some() {
                data[offset..offset + FRAME].copy_from_slice(&pixels);
            } else {
                for y in 0..HEIGHT {
                    let start = offset + y * row.len();
                    data[start..start + row.len()].copy_from_slice(&row);
                }
            }
            data[24..32].copy_from_slice(&sequence.to_le_bytes());
        }
        timings.add(render_elapsed, readback_elapsed, publish_started.elapsed());
        if sequence % (FPS * 5) == 0 {
            println!("[KDE TEST] published={sequence}, average FPS={:.1}; mean render={:.2}ms, readback={:.2}ms, publish={:.2}ms",
                sequence as f64 / started.elapsed().as_secs_f64(),
                timings.mean_ms(timings.render), timings.mean_ms(timings.readback), timings.mean_ms(timings.publish));
        }
        deadline += Duration::from_nanos(1_000_000_000 / FPS);
        let now = Instant::now();
        if deadline > now { thread::sleep(deadline - now); }
        else { deadline = now; }
    }
    // Drop mapping before removing the pathname. The C++ consumer detects its
    // disappearance and reconnects on the next producer run.
    drop(renderer);
    drop(mapping);
    drop(file);
    fs::remove_file(&bin_path).map_err(|e| format!("remove frame file: {e}"))?;
    println!("[KDE TEST] stopped after {sequence} frames.");
    Ok(())
}


#[derive(Default)]
struct Timings {
    count: u64,
    render: Duration,
    readback: Duration,
    publish: Duration,
}
impl Timings {
    fn add(&mut self, render: Duration, readback: Duration, publish: Duration) {
        self.count += 1;
        self.render += render;
        self.readback += readback;
        self.publish += publish;
    }
    fn mean_ms(&self, total: Duration) -> f64 {
        total.as_secs_f64() * 1000.0 / self.count.max(1) as f64
    }
}

// The SDL window exists only to own a GL context. Nothing is presented to it.
struct OffscreenRenderer {
    _context: sdl2::video::GLContext,
    _window: sdl2::video::Window,
    _video: sdl2::VideoSubsystem,
    _sdl: sdl2::Sdl,
    program: u32,
    vao: u32,
    framebuffer: u32,
    texture: u32,
    pixels: Vec<u8>,
    output: Vec<u8>,
    last_time: f32,
}
impl OffscreenRenderer {
    fn new(path: &str) -> Result<Self, String> {
        use sdl2::video::GLProfile;
        let sdl = sdl2::init().map_err(|e| format!("SDL init: {e}"))?;
        let video = sdl.video().map_err(|e| format!("SDL video: {e}"))?;
        {
            let attr = video.gl_attr();
            attr.set_context_profile(GLProfile::Core);
            attr.set_context_version(crate::define_constants::GL_MAJOR, crate::define_constants::GL_MINOR);
        }
        let window = video.window("Screenshaver KDE offscreen test", 64, 64)
            .opengl().hidden().build().map_err(|e| format!("GL window: {e}"))?;
        let context = window.gl_create_context().map_err(|e| format!("GL context: {e}"))?;
        gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
        let source = match crate::load_shader::load_shader_for_preview(std::path::Path::new(path)) {
            crate::load_shader::ShaderLoadResult::Ready { source, channel_usage, shader_inputs, .. } => {
                if channel_usage.uses_any_channel() || !shader_inputs.is_empty() {
                    return Err("Phase 2 supports shaders without texture channels or ISF inputs".into());
                }
                source
            }
            crate::load_shader::ShaderLoadResult::Rejected { reasons, .. } =>
                return Err(format!("Shader rejected: {}", reasons.join("; "))),
            crate::load_shader::ShaderLoadResult::Unavailable { error, .. } =>
                return Err(format!("Shader unavailable: {error}")),
        };
        let vertex = r#"#version 330 core
const vec2 positions[3] = vec2[3](vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
void main() { gl_Position = vec4(positions[gl_VertexID], 0.0, 1.0); }
"#;
        let program = crate::compile_shader::build_program(vertex, &source)?;
        let (mut vao, mut framebuffer, mut texture) = (0_u32, 0_u32, 0_u32);
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32,
                WIDTH as i32, HEIGHT as i32, 0, gl::RGBA, gl::UNSIGNED_BYTE, std::ptr::null());
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture, 0);
            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                gl::DeleteTextures(1, &texture);
                gl::DeleteFramebuffers(1, &framebuffer);
                gl::DeleteVertexArrays(1, &vao);
                gl::DeleteProgram(program);
                return Err("OpenGL offscreen framebuffer incomplete".into());
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        Ok(Self { _context: context, _window: window, _video: video, _sdl: sdl,
            program, vao, framebuffer, texture, pixels: vec![0; FRAME],
            output: vec![0; FRAME], last_time: 0.0 })
    }
    fn render(&mut self, elapsed: f32, sequence: u64) -> Result<(Vec<u8>, Duration, Duration), String> {
        let render_start = Instant::now();
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);
            gl::Viewport(0, 0, WIDTH as i32, HEIGHT as i32);
            gl::UseProgram(self.program);
            self.uniform_1f(b"iTime\0", elapsed);
            self.uniform_1f(b"iTimeDelta\0", elapsed - self.last_time);
            self.uniform_1i(b"iFrame\0", (sequence - 1).min(i32::MAX as u64) as i32);
            let loc = gl::GetUniformLocation(self.program, b"iResolution\0".as_ptr().cast());
            if loc >= 0 { gl::Uniform3f(loc, WIDTH as f32, HEIGHT as f32, 1.0); }
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }
        self.last_time = elapsed;
        let render_elapsed = render_start.elapsed();
        let read_start = Instant::now();
        unsafe {
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::ReadPixels(0, 0, WIDTH as i32, HEIGHT as i32,
                gl::RGBA, gl::UNSIGNED_BYTE, self.pixels.as_mut_ptr().cast());
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        // GL starts at the bottom-left; QImage's scanlines start at the top-left.
        let stride = WIDTH * 4;
        for y in 0..HEIGHT {
            let from = (HEIGHT - 1 - y) * stride;
            let to = y * stride;
            self.output[to..to + stride].copy_from_slice(&self.pixels[from..from + stride]);
        }
        let read_elapsed = read_start.elapsed();
        Ok((self.output.clone(), render_elapsed, read_elapsed))
    }
    unsafe fn uniform_1f(&self, name: &[u8], value: f32) {
        let loc = gl::GetUniformLocation(self.program, name.as_ptr().cast());
        if loc >= 0 { gl::Uniform1f(loc, value); }
    }
    unsafe fn uniform_1i(&self, name: &[u8], value: i32) {
        let loc = gl::GetUniformLocation(self.program, name.as_ptr().cast());
        if loc >= 0 { gl::Uniform1i(loc, value); }
    }
}
impl Drop for OffscreenRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteFramebuffers(1, &self.framebuffer);
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteProgram(self.program);
        }
    }
}
