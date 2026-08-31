use std::ffi::c_void;
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use sdl2::video::GLProfile;

use crate::render_frame::FrameRenderEngine;

const TRANSPORT_WIDTH: u32 = 1280;
const TRANSPORT_HEIGHT: u32 = 720;
const TRANSPORT_ROWSTRIDE: u32 = TRANSPORT_WIDTH * 4;
const TRANSPORT_FRAME_INTERVAL: Duration = Duration::from_millis(100);
const TRANSPORT_FILENAME: &str = "screenshaver-lock-frame.shm";

const TRANSPORT_MAGIC: [u8; 8] = *b"SHVRGNM1";
const TRANSPORT_VERSION: u32 = 1;
const TRANSPORT_HEADER_BYTES: usize = 64;
const TRANSPORT_SLOT_COUNT: u32 = 2;

const HEADER_MAGIC_OFFSET: usize = 0;
const HEADER_VERSION_OFFSET: usize = 8;
const HEADER_SIZE_OFFSET: usize = 12;
const HEADER_WIDTH_OFFSET: usize = 16;
const HEADER_HEIGHT_OFFSET: usize = 20;
const HEADER_ROWSTRIDE_OFFSET: usize = 24;
const HEADER_FRAME_BYTES_OFFSET: usize = 28;
const HEADER_SLOT_COUNT_OFFSET: usize = 32;
const HEADER_ACTIVE_SLOT_OFFSET: usize = 36;
const HEADER_FRAME_COUNTER_OFFSET: usize = 40;

#[cfg(unix)]
const PROT_READ: i32 = 0x1;
#[cfg(unix)]
const PROT_WRITE: i32 = 0x2;
#[cfg(unix)]
const MAP_SHARED: i32 = 0x01;

#[cfg(unix)]
unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: isize,
    ) -> *mut c_void;

    fn munmap(addr: *mut c_void, length: usize) -> i32;
}

/// GNOME lock-screen presentation host.
///
/// SDL and OpenGL ownership remain on Screenshaver's main thread. GNOME's
/// blocking secure-lock D-Bus wait runs on a worker thread in `main.rs` while
/// this presenter renders frames continuously.
///
/// Frame transport is a small versioned, double-buffered shared-memory region
/// backed by a file in `$XDG_RUNTIME_DIR`. The file is created once for the
/// lock session and memory-mapped by both Screenshaver and the GNOME Shell
/// extension. Per-frame create/write/rename I/O is no longer used.
pub(crate) struct GnomeLockPresenter {
    producer: GnomeLockFrameProducer,
}

impl GnomeLockPresenter {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start(
        sdl: &sdl2::Sdl,
        logfile: &Path,
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy: crate::load_config::AnimationSpeedPolicy,
        global_rendered_fps: u32,
        fps_policy_entries: Vec<crate::load_config::FpsPolicyEntry>,
        texture_policy: crate::load_config::TexturePolicy,
        postprocess_policy: crate::load_config::PostprocessPolicy,
        audio_bands: Option<crate::audio_backend::SharedAudioBands>,
        subtitles: bool,
        subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        let producer = GnomeLockFrameProducer::new(
            sdl,
            logfile,
            shader_manager,
            shader_interval,
            animation_speed_policy,
            global_rendered_fps,
            fps_policy_entries,
            texture_policy,
            postprocess_policy,
            audio_bands,
            subtitles,
            subtitle_placement,
        )?;

        log_information(
            logfile,
            "[LOCK] GNOME shader presentation backend initialized on main SDL thread",
        );

        Ok(Self { producer })
    }

    /// Render until the caller reports that GNOME's secure-lock worker has
    /// completed. This method must remain on the same thread that owns `sdl`.
    pub(crate) fn run_until<F>(
        &mut self,
        mut lock_finished: F,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        self.producer.run_until(&mut lock_finished)
    }
}

struct GnomeLockFrameProducer {
    logfile: PathBuf,

    // Keep the render engine before its OpenGL context/window so engine-owned
    // GL resources are dropped while the context is still alive.
    engine: FrameRenderEngine,
    _gl_context: sdl2::video::GLContext,
    window: sdl2::video::Window,

    transport: SharedFrameTransport,
    readback: Vec<u8>,
    top_down_rgba: Vec<u8>,
    last_publish: Instant,
    published_frames: u64,
}

impl GnomeLockFrameProducer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        sdl: &sdl2::Sdl,
        logfile: &Path,
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy: crate::load_config::AnimationSpeedPolicy,
        global_rendered_fps: u32,
        fps_policy_entries: Vec<crate::load_config::FpsPolicyEntry>,
        texture_policy: crate::load_config::TexturePolicy,
        postprocess_policy: crate::load_config::PostprocessPolicy,
        audio_bands: Option<crate::audio_backend::SharedAudioBands>,
        subtitles: bool,
        subtitle_placement: crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        log_information(
            logfile,
            "[LOCK] Initializing GNOME hidden shader presentation surface on main SDL thread",
        );

        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| {
                "XDG_RUNTIME_DIR is unavailable for GNOME lock frame transport"
                    .to_string()
            })?;

        let transport_path = runtime_dir.join(TRANSPORT_FILENAME);

        // Reuse the SDL instance that main.rs already initialized. Creating a
        // second SDL instance on another thread is rejected by rust-sdl2.
        let video = sdl
            .video()
            .map_err(|error| {
                format!(
                    "Failed to access SDL video for GNOME lock presentation: {error}"
                )
            })?;

        {
            let gl_attr = video.gl_attr();
            gl_attr.set_context_profile(GLProfile::Core);
            gl_attr.set_context_version(
                crate::define_constants::GL_MAJOR,
                crate::define_constants::GL_MINOR,
            );
        }

        let window = video
            .window(
                "Screenshaver GNOME Lock Renderer",
                TRANSPORT_WIDTH,
                TRANSPORT_HEIGHT,
            )
            .position_centered()
            .hidden()
            .opengl()
            .build()
            .map_err(|error| {
                format!(
                    "Failed to create hidden GNOME lock renderer window: {error}"
                )
            })?;

        let gl_context = window
            .gl_create_context()
            .map_err(|error| {
                format!("Failed to create GNOME lock OpenGL context: {error}")
            })?;

        window
            .gl_make_current(&gl_context)
            .map_err(|error| {
                format!("Failed to activate GNOME lock OpenGL context: {error}")
            })?;

        gl::load_with(|symbol| video.gl_get_proc_address(symbol) as *const _);

        let _ = video.gl_set_swap_interval(0);

        let engine = FrameRenderEngine::new(
            shader_manager,
            shader_interval,
            animation_speed_policy,
            global_rendered_fps,
            fps_policy_entries,
            texture_policy,
            postprocess_policy,
            audio_bands,
            subtitles,
            subtitle_placement,
            TRANSPORT_WIDTH,
            TRANSPORT_HEIGHT,
        )?;

        let frame_bytes = frame_bytes()?;
        let transport = SharedFrameTransport::create(
            transport_path,
            frame_bytes,
        )?;

        log_information(
            logfile,
            &format!(
                "[LOCK] GNOME hidden shader surface ready: {}x{}; shared-memory transport={}",
                TRANSPORT_WIDTH,
                TRANSPORT_HEIGHT,
                transport.path().display(),
            ),
        );

        Ok(Self {
            logfile: logfile.to_path_buf(),
            engine,
            _gl_context: gl_context,
            window,
            transport,
            readback: vec![0; frame_bytes],
            top_down_rgba: vec![0; frame_bytes],
            last_publish: Instant::now() - TRANSPORT_FRAME_INTERVAL,
            published_frames: 0,
        })
    }

    fn run_until<F>(
        &mut self,
        lock_finished: &mut F,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        log_information(
            &self.logfile,
            "[LOCK] GNOME shader presentation render loop started",
        );

        let render_result = self.render_loop(lock_finished);

        self.transport.remove_backing_file();

        log_information(
            &self.logfile,
            "[LOCK] GNOME shader presentation render loop stopped",
        );

        render_result
    }

    fn render_loop<F>(
        &mut self,
        lock_finished: &mut F,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        while !lock_finished() {
            let _ = self
                .engine
                .render_frame(TRANSPORT_WIDTH, TRANSPORT_HEIGHT);

            if self.last_publish.elapsed() >= TRANSPORT_FRAME_INTERVAL {
                self.capture_frame();
                self.transport.publish(&self.top_down_rgba)?;
                self.last_publish = Instant::now();
                self.published_frames = self.published_frames.saturating_add(1);

                if self.published_frames % 50 == 0 {
                    log_information(
                        &self.logfile,
                        &format!(
                            "[LOCK] GNOME shared-memory shader frames published: {}",
                            self.published_frames,
                        ),
                    );
                }
            }

            self.window.gl_swap_window();
            self.engine.limit_fps();
        }

        Ok(())
    }

    fn capture_frame(&mut self) {
        unsafe {
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::ReadBuffer(gl::BACK);
            gl::ReadPixels(
                0,
                0,
                TRANSPORT_WIDTH as i32,
                TRANSPORT_HEIGHT as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.readback.as_mut_ptr() as *mut _,
            );
        }

        // OpenGL's framebuffer origin is bottom-left. St.ImageContent expects
        // the first row to represent the top of the image, so reverse the row
        // order before publishing the transport frame.
        let row_bytes = TRANSPORT_ROWSTRIDE as usize;

        for destination_row in 0..TRANSPORT_HEIGHT as usize {
            let source_row = TRANSPORT_HEIGHT as usize - 1 - destination_row;

            let source_start = source_row * row_bytes;
            let destination_start = destination_row * row_bytes;

            self.top_down_rgba
                [destination_start..destination_start + row_bytes]
                .copy_from_slice(
                    &self.readback[source_start..source_start + row_bytes],
                );
        }
    }
}

struct SharedFrameTransport {
    path: PathBuf,
    _file: File,
    mapping: *mut u8,
    mapping_len: usize,
    frame_bytes: usize,
    active_slot: u32,
    frame_counter: u32,
    removed: bool,
}

impl SharedFrameTransport {
    fn create(
        path: PathBuf,
        frame_bytes: usize,
    ) -> Result<Self, String> {
        #[cfg(not(unix))]
        {
            let _ = path;
            let _ = frame_bytes;
            return Err(
                "GNOME shared-memory lock transport requires a Unix platform"
                    .to_string(),
            );
        }

        #[cfg(unix)]
        {
            let mapping_len = TRANSPORT_HEADER_BYTES
                .checked_add(
                    frame_bytes
                        .checked_mul(TRANSPORT_SLOT_COUNT as usize)
                        .ok_or_else(|| {
                            "GNOME lock shared-memory transport size overflow"
                                .to_string()
                        })?,
                )
                .ok_or_else(|| {
                    "GNOME lock shared-memory transport size overflow"
                        .to_string()
                })?;

            let _ = fs::remove_file(&path);

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .map_err(|error| {
                    format!(
                        "Unable to create GNOME lock shared-memory backing file '{}': {error}",
                        path.display(),
                    )
                })?;

            file.set_len(mapping_len as u64)
                .map_err(|error| {
                    format!(
                        "Unable to size GNOME lock shared-memory backing file '{}': {error}",
                        path.display(),
                    )
                })?;

            let mapped = unsafe {
                mmap(
                    ptr::null_mut(),
                    mapping_len,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED,
                    file.as_raw_fd(),
                    0,
                )
            };

            if mapped as isize == -1 {
                let _ = fs::remove_file(&path);
                return Err(format!(
                    "Unable to memory-map GNOME lock transport '{}': {}",
                    path.display(),
                    std::io::Error::last_os_error(),
                ));
            }

            let mapping = mapped.cast::<u8>();

            unsafe {
                ptr::write_bytes(mapping, 0, mapping_len);
            }

            let mut transport = Self {
                path,
                _file: file,
                mapping,
                mapping_len,
                frame_bytes,
                active_slot: 0,
                frame_counter: 0,
                removed: false,
            };

            transport.initialize_header();

            Ok(transport)
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn initialize_header(&mut self) {
        self.write_bytes(HEADER_MAGIC_OFFSET, &TRANSPORT_MAGIC);
        self.write_u32(HEADER_VERSION_OFFSET, TRANSPORT_VERSION);
        self.write_u32(
            HEADER_SIZE_OFFSET,
            TRANSPORT_HEADER_BYTES as u32,
        );
        self.write_u32(HEADER_WIDTH_OFFSET, TRANSPORT_WIDTH);
        self.write_u32(HEADER_HEIGHT_OFFSET, TRANSPORT_HEIGHT);
        self.write_u32(HEADER_ROWSTRIDE_OFFSET, TRANSPORT_ROWSTRIDE);
        self.write_u32(
            HEADER_FRAME_BYTES_OFFSET,
            self.frame_bytes as u32,
        );
        self.write_u32(HEADER_SLOT_COUNT_OFFSET, TRANSPORT_SLOT_COUNT);

        self.active_slot_atomic()
            .store(self.active_slot, Ordering::Release);
        self.frame_counter_atomic()
            .store(self.frame_counter, Ordering::Release);
    }

    fn publish(&mut self, rgba: &[u8]) -> Result<(), String> {
        if rgba.len() != self.frame_bytes {
            return Err(format!(
                "GNOME shared-memory frame has {} bytes; expected {}",
                rgba.len(),
                self.frame_bytes,
            ));
        }

        let next_slot = if self.active_slot == 0 { 1 } else { 0 };
        let slot_offset = TRANSPORT_HEADER_BYTES
            + next_slot as usize * self.frame_bytes;

        self.write_bytes(slot_offset, rgba);

        // Publish the completed inactive slot only after its pixel copy is
        // finished. The extension reads the monotonically increasing frame
        // counter and uploads only when it observes a new completed frame.
        self.active_slot = next_slot;
        self.frame_counter = self.frame_counter.wrapping_add(1);

        self.active_slot_atomic()
            .store(self.active_slot, Ordering::Release);
        self.frame_counter_atomic()
            .store(self.frame_counter, Ordering::Release);

        Ok(())
    }

    fn remove_backing_file(&mut self) {
        if self.removed {
            return;
        }

        let _ = fs::remove_file(&self.path);
        self.removed = true;
    }

    fn write_u32(&mut self, offset: usize, value: u32) {
        self.write_bytes(offset, &value.to_le_bytes());
    }

    fn write_bytes(&mut self, offset: usize, bytes: &[u8]) {
        debug_assert!(offset + bytes.len() <= self.mapping_len);

        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.mapping.add(offset),
                bytes.len(),
            );
        }
    }

    fn active_slot_atomic(&self) -> &AtomicU32 {
        unsafe {
            &*(self.mapping.add(HEADER_ACTIVE_SLOT_OFFSET)
                as *const AtomicU32)
        }
    }

    fn frame_counter_atomic(&self) -> &AtomicU32 {
        unsafe {
            &*(self.mapping.add(HEADER_FRAME_COUNTER_OFFSET)
                as *const AtomicU32)
        }
    }
}

impl Drop for SharedFrameTransport {
    fn drop(&mut self) {
        self.remove_backing_file();

        #[cfg(unix)]
        unsafe {
            if !self.mapping.is_null() && self.mapping_len != 0 {
                let _ = munmap(
                    self.mapping.cast::<c_void>(),
                    self.mapping_len,
                );
            }
        }
    }
}

fn frame_bytes() -> Result<usize, String> {
    (TRANSPORT_ROWSTRIDE as usize)
        .checked_mul(TRANSPORT_HEIGHT as usize)
        .ok_or_else(|| "GNOME lock RGBA frame size overflow".to_string())
}

fn log_information(logfile: &Path, message: &str) {
    crate::logger::information(logfile, message);
}
