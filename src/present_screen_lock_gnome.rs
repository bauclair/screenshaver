use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sdl2::video::GLProfile;

use crate::monitor_lock_presentation::{
    LockPresentationBackend,
    LockPresentationMonitor,
    LockPresentationSample,
};
use crate::render_frame::FrameRenderEngine;

const TRANSPORT_WIDTH: u32 = 1920;
const TRANSPORT_HEIGHT: u32 = 1080;
const TRANSPORT_ROWSTRIDE: u32 = TRANSPORT_WIDTH * 4;
const CONTROL_FILENAME: &str = "screenshaver-lock-control.bin";
const FRAME_FILENAME_PREFIX: &str = "screenshaver-lock-frame-";
const FRAME_FILENAME_SUFFIX: &str = ".rgba";
const CONTROL_MAGIC: [u8; 8] = *b"SHVRGNF1";
const CONTROL_VERSION: u32 = 1;
const CONTROL_BYTES: usize = 64;
const CONTROL_SESSION_ID_BYTES: usize = 16;
const RETAINED_FRAME_COUNT: u32 = 32;
const TRANSPORT_PUBLISH_FPS: u32 = 10;
const TRANSPORT_PUBLISH_INTERVAL: Duration =
    Duration::from_millis(1000 / TRANSPORT_PUBLISH_FPS as u64);

const CONTROL_MAGIC_OFFSET: usize = 0;
const CONTROL_VERSION_OFFSET: usize = 8;
const CONTROL_SIZE_OFFSET: usize = 12;
const CONTROL_WIDTH_OFFSET: usize = 16;
const CONTROL_HEIGHT_OFFSET: usize = 20;
const CONTROL_ROWSTRIDE_OFFSET: usize = 24;
const CONTROL_FRAME_BYTES_OFFSET: usize = 28;
const CONTROL_FRAME_COUNTER_OFFSET: usize = 32;
const CONTROL_SESSION_ID_OFFSET: usize = 36;

/// GNOME lock-screen presentation host.
///
/// SDL and OpenGL ownership remain on Screenshaver's main thread. GNOME's
/// blocking secure-lock D-Bus wait runs on a worker thread in `main.rs` while
/// this presenter renders frames continuously.
///
/// Frame transport uses immutable completed RGBA frame files plus a tiny
/// atomically replaced control record in `$XDG_RUNTIME_DIR`. GNOME Shell never
/// observes a live mutable mmap: it polls only the small control record and
/// asynchronously reads a completed frame when the published counter changes.
pub(crate) struct GnomeLockPresenter {
    producer: GnomeLockFrameProducer,
}

impl GnomeLockPresenter {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start(
        sdl: &sdl2::Sdl,
        logfile: &Path,
        session_id: &str,
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
            session_id,
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

    transport: FileFrameTransport,
    readback: Vec<u8>,
    top_down_rgba: Vec<u8>,
    published_frames: u64,
    last_transport_publish: Option<Instant>,
    presentation_monitor: LockPresentationMonitor,
}

impl GnomeLockFrameProducer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        sdl: &sdl2::Sdl,
        logfile: &Path,
        session_id: &str,
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

        let control_path = runtime_dir.join(CONTROL_FILENAME);

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

        let session_id_bytes =
            decode_session_id(
                session_id
            )?;

        let frame_bytes = frame_bytes()?;
        let transport = FileFrameTransport::create(
            runtime_dir,
            control_path,
            frame_bytes,
            session_id_bytes,
        )?;

        log_information(
            logfile,
            &format!(
                "[LOCK] GNOME hidden shader surface ready: {}x{}; file transport control={}",
                TRANSPORT_WIDTH,
                TRANSPORT_HEIGHT,
                transport.control_path().display(),
            ),
        );

        let presentation_monitor = LockPresentationMonitor::new(
            LockPresentationBackend::Gnome,
            logfile,
            engine.current_metadata().configured_fps,
        );

        Ok(Self {
            logfile: logfile.to_path_buf(),
            engine,
            _gl_context: gl_context,
            window,
            transport,
            readback: vec![0; frame_bytes],
            top_down_rgba: vec![0; frame_bytes],
            published_frames: 0,
            last_transport_publish: None,
            presentation_monitor,
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

        self.transport.cleanup();

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

            // Render at the policy-configured rate, but publish completed RGBA
            // frames to GNOME at a deliberately lower transport cadence.  This
            // diagnostic keeps shader timing unchanged while reducing file I/O,
            // memory copies, async reads, and texture uploads to about 10 FPS.
            let should_publish = self
                .last_transport_publish
                .map(|last| last.elapsed() >= TRANSPORT_PUBLISH_INTERVAL)
                .unwrap_or(true);

            if should_publish {
                // FrameRenderEngine owns the existing shader/render performance
                // accounting. Measure only the GNOME-specific presentation work
                // that begins after render_frame() returns.
                let presentation_started = Instant::now();

                let (readback_elapsed, row_flip_elapsed) =
                    self.capture_frame();

                let transfer_started = Instant::now();
                self.transport.publish(&self.top_down_rgba)?;
                let transfer_elapsed = transfer_started.elapsed();

                self.last_transport_publish = Some(Instant::now());
                self.published_frames = self.published_frames.saturating_add(1);

                if self.published_frames % 50 == 0 {
                    log_information(
                        &self.logfile,
                        &format!(
                            "[LOCK] GNOME file-transport shader frames published: {}",
                            self.published_frames,
                        ),
                    );
                }

                let submit_started = Instant::now();
                self.window.gl_swap_window();
                let submit_elapsed = submit_started.elapsed();

                let configured_fps =
                    self.engine.current_metadata().configured_fps;

                self.presentation_monitor.record(
                    LockPresentationSample {
                        configured_fps,
                        readback: readback_elapsed,
                        row_flip: row_flip_elapsed,
                        transfer: transfer_elapsed,
                        submit: submit_elapsed,
                        total: presentation_started.elapsed(),
                    },
                );
            }

            self.engine.limit_fps();
        }

        Ok(())
    }

    fn capture_frame(&mut self) -> (std::time::Duration, std::time::Duration) {
        let readback_started = Instant::now();

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

        let readback_elapsed = readback_started.elapsed();
        let row_flip_started = Instant::now();

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

        let row_flip_elapsed = row_flip_started.elapsed();

        (readback_elapsed, row_flip_elapsed)
    }
}

struct FileFrameTransport {
    runtime_dir: PathBuf,
    control_path: PathBuf,
    control_temp_path: PathBuf,
    frame_bytes: usize,
    session_id: [u8; CONTROL_SESSION_ID_BYTES],
    frame_counter: u32,
    removed: bool,
}

impl FileFrameTransport {
    fn create(
        runtime_dir: PathBuf,
        control_path: PathBuf,
        frame_bytes: usize,
        session_id: [u8; CONTROL_SESSION_ID_BYTES],
    ) -> Result<Self, String> {
        let control_temp_path = runtime_dir.join(format!("{CONTROL_FILENAME}.tmp"));
        let _ = fs::remove_file(&control_path);
        let _ = fs::remove_file(&control_temp_path);

        // Remove stale immutable frame files left by an interrupted prior run.
        if let Ok(entries) = fs::read_dir(&runtime_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(FRAME_FILENAME_PREFIX)
                    && (name.ends_with(FRAME_FILENAME_SUFFIX)
                        || name.ends_with(".rgba.tmp"))
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        let mut transport = Self {
            runtime_dir,
            control_path,
            control_temp_path,
            frame_bytes,
            session_id,
            frame_counter: 0,
            removed: false,
        };

        transport.publish_control(0)?;
        Ok(transport)
    }

    fn control_path(&self) -> &Path {
        &self.control_path
    }

    fn publish(&mut self, rgba: &[u8]) -> Result<(), String> {
        if rgba.len() != self.frame_bytes {
            return Err(format!(
                "GNOME file-transport frame has {} bytes; expected {}",
                rgba.len(),
                self.frame_bytes,
            ));
        }

        self.frame_counter = self.frame_counter.wrapping_add(1);
        if self.frame_counter == 0 {
            self.frame_counter = 1;
        }
        let counter = self.frame_counter;

        let frame_path = self.frame_path(counter);
        let frame_temp_path = self.frame_temp_path(counter);
        let _ = fs::remove_file(&frame_temp_path);

        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&frame_temp_path)
                .map_err(|error| {
                    format!(
                        "Unable to create GNOME lock frame '{}': {error}",
                        frame_temp_path.display(),
                    )
                })?;
            file.write_all(rgba).map_err(|error| {
                format!(
                    "Unable to write GNOME lock frame '{}': {error}",
                    frame_temp_path.display(),
                )
            })?;
            file.flush().map_err(|error| {
                format!(
                    "Unable to flush GNOME lock frame '{}': {error}",
                    frame_temp_path.display(),
                )
            })?;
        }

        fs::rename(&frame_temp_path, &frame_path).map_err(|error| {
            format!(
                "Unable to publish GNOME lock frame '{}' -> '{}': {error}",
                frame_temp_path.display(),
                frame_path.display(),
            )
        })?;

        // Publish the tiny control record only after the immutable frame has
        // been completely written and renamed into place.
        self.publish_control(counter)?;

        if counter > RETAINED_FRAME_COUNT {
            let obsolete = counter - RETAINED_FRAME_COUNT;
            let _ = fs::remove_file(self.frame_path(obsolete));
        }

        Ok(())
    }

    fn publish_control(&mut self, frame_counter: u32) -> Result<(), String> {
        let mut control = [0u8; CONTROL_BYTES];
        control[CONTROL_MAGIC_OFFSET..CONTROL_MAGIC_OFFSET + CONTROL_MAGIC.len()]
            .copy_from_slice(&CONTROL_MAGIC);
        write_u32_slice(&mut control, CONTROL_VERSION_OFFSET, CONTROL_VERSION);
        write_u32_slice(&mut control, CONTROL_SIZE_OFFSET, CONTROL_BYTES as u32);
        write_u32_slice(&mut control, CONTROL_WIDTH_OFFSET, TRANSPORT_WIDTH);
        write_u32_slice(&mut control, CONTROL_HEIGHT_OFFSET, TRANSPORT_HEIGHT);
        write_u32_slice(&mut control, CONTROL_ROWSTRIDE_OFFSET, TRANSPORT_ROWSTRIDE);
        write_u32_slice(&mut control, CONTROL_FRAME_BYTES_OFFSET, self.frame_bytes as u32);
        write_u32_slice(&mut control, CONTROL_FRAME_COUNTER_OFFSET, frame_counter);
        control[CONTROL_SESSION_ID_OFFSET
            ..CONTROL_SESSION_ID_OFFSET + CONTROL_SESSION_ID_BYTES]
            .copy_from_slice(&self.session_id);

        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.control_temp_path)
                .map_err(|error| {
                    format!(
                        "Unable to create GNOME lock control record '{}': {error}",
                        self.control_temp_path.display(),
                    )
                })?;
            file.write_all(&control).map_err(|error| {
                format!(
                    "Unable to write GNOME lock control record '{}': {error}",
                    self.control_temp_path.display(),
                )
            })?;
            file.flush().map_err(|error| {
                format!(
                    "Unable to flush GNOME lock control record '{}': {error}",
                    self.control_temp_path.display(),
                )
            })?;
        }

        fs::rename(&self.control_temp_path, &self.control_path).map_err(|error| {
            format!(
                "Unable to publish GNOME lock control record '{}': {error}",
                self.control_path.display(),
            )
        })
    }

    fn frame_path(&self, counter: u32) -> PathBuf {
        self.runtime_dir.join(format!(
            "{FRAME_FILENAME_PREFIX}{counter:010}{FRAME_FILENAME_SUFFIX}"
        ))
    }

    fn frame_temp_path(&self, counter: u32) -> PathBuf {
        self.runtime_dir.join(format!(
            "{FRAME_FILENAME_PREFIX}{counter:010}{FRAME_FILENAME_SUFFIX}.tmp"
        ))
    }

    fn cleanup(&mut self) {
        if self.removed {
            return;
        }

        let _ = fs::remove_file(&self.control_path);
        let _ = fs::remove_file(&self.control_temp_path);

        if let Ok(entries) = fs::read_dir(&self.runtime_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(FRAME_FILENAME_PREFIX)
                    && (name.ends_with(FRAME_FILENAME_SUFFIX)
                        || name.ends_with(".rgba.tmp"))
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        self.removed = true;
    }
}

impl Drop for FileFrameTransport {
    fn drop(&mut self) {
        self.cleanup();
    }
}

fn write_u32_slice(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn decode_session_id(
    session_id: &str,
) -> Result<[u8; CONTROL_SESSION_ID_BYTES], String> {

    if session_id.len()
        != CONTROL_SESSION_ID_BYTES * 2
    {
        return Err(
            format!(
                "GNOME runtime session identity has {} hexadecimal characters; expected {}",
                session_id.len(),
                CONTROL_SESSION_ID_BYTES * 2,
            )
        );
    }


    let mut decoded =
        [0u8; CONTROL_SESSION_ID_BYTES];

    let bytes =
        session_id.as_bytes();


    for index in 0..CONTROL_SESSION_ID_BYTES {
        let high =
            decode_hex_nibble(
                bytes[index * 2]
            )
            .ok_or_else(
                || {
                    "GNOME runtime session identity contains a non-hexadecimal character"
                        .to_string()
                }
            )?;

        let low =
            decode_hex_nibble(
                bytes[index * 2 + 1]
            )
            .ok_or_else(
                || {
                    "GNOME runtime session identity contains a non-hexadecimal character"
                        .to_string()
                }
            )?;

        decoded[index] =
            (high << 4) | low;
    }


    Ok(
        decoded
    )
}


fn decode_hex_nibble(
    byte: u8,
) -> Option<u8> {

    match byte {
        b'0'..=b'9' => {
            Some(
                byte - b'0'
            )
        }

        b'a'..=b'f' => {
            Some(
                byte - b'a' + 10
            )
        }

        b'A'..=b'F' => {
            Some(
                byte - b'A' + 10
            )
        }

        _ => {
            None
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
