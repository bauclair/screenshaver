use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sdl2::video::GLProfile;

use crate::render_frame::FrameRenderEngine;

const TRANSPORT_WIDTH: u32 = 640;
const TRANSPORT_HEIGHT: u32 = 360;
const TRANSPORT_FRAME_INTERVAL: Duration = Duration::from_millis(100);
const TRANSPORT_FILENAME: &str = "screenshaver-lock-test.rgba";
const TRANSPORT_TEMP_FILENAME: &str = ".screenshaver-lock-test.rgba.tmp";

/// Temporary GNOME lock-screen presentation host.
///
/// This proof-of-integration host deliberately keeps SDL and OpenGL ownership
/// on Screenshaver's main thread. GNOME's blocking secure-lock D-Bus wait runs
/// on a separate worker thread in `main.rs` while this presenter continuously
/// renders and publishes frames for the GNOME Shell extension.
///
/// Completed RGBA frames are still atomically published in `$XDG_RUNTIME_DIR`.
/// That file transport is temporary and will be replaced after real
/// Screenshaver shader output is proven behind GNOME's secure lock UI.
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

    frame_path: PathBuf,
    temp_path: PathBuf,
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

        let frame_path = runtime_dir.join(TRANSPORT_FILENAME);
        let temp_path = runtime_dir.join(TRANSPORT_TEMP_FILENAME);

        let _ = fs::remove_file(&frame_path);
        let _ = fs::remove_file(&temp_path);

        // Reuse the SDL instance that main.rs already initialized. Creating a
        // second SDL instance on a render worker thread is rejected by rust-sdl2.
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

        let frame_bytes = (TRANSPORT_WIDTH as usize)
            .checked_mul(TRANSPORT_HEIGHT as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "GNOME lock RGBA frame size overflow".to_string())?;

        log_information(
            logfile,
            &format!(
                "[LOCK] GNOME hidden shader surface ready: {}x{}; transport={}",
                TRANSPORT_WIDTH,
                TRANSPORT_HEIGHT,
                frame_path.display(),
            ),
        );

        Ok(Self {
            logfile: logfile.to_path_buf(),
            engine,
            _gl_context: gl_context,
            window,
            frame_path,
            temp_path,
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

        self.cleanup_transport();

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
                self.publish_frame()?;
                self.last_publish = Instant::now();
                self.published_frames = self.published_frames.saturating_add(1);

                if self.published_frames % 50 == 0 {
                    log_information(
                        &self.logfile,
                        &format!(
                            "[LOCK] GNOME shader frames published: {}",
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
        let row_bytes = TRANSPORT_WIDTH as usize * 4;

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

    fn publish_frame(&self) -> Result<(), String> {
        let mut file = File::create(&self.temp_path).map_err(|error| {
            format!(
                "Unable to create temporary GNOME lock frame '{}': {error}",
                self.temp_path.display(),
            )
        })?;

        file.write_all(&self.top_down_rgba).map_err(|error| {
            format!(
                "Unable to write temporary GNOME lock frame '{}': {error}",
                self.temp_path.display(),
            )
        })?;

        file.flush().map_err(|error| {
            format!(
                "Unable to flush temporary GNOME lock frame '{}': {error}",
                self.temp_path.display(),
            )
        })?;

        drop(file);

        fs::rename(&self.temp_path, &self.frame_path).map_err(|error| {
            format!(
                "Unable to publish GNOME lock frame '{}': {error}",
                self.frame_path.display(),
            )
        })?;

        Ok(())
    }

    fn cleanup_transport(&self) {
        let _ = fs::remove_file(&self.temp_path);
        let _ = fs::remove_file(&self.frame_path);
    }
}

impl Drop for GnomeLockFrameProducer {
    fn drop(&mut self) {
        self.cleanup_transport();
    }
}

fn log_information(logfile: &Path, message: &str) {
    crate::logger::information(logfile, message);
}
