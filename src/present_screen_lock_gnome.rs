use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sdl2::video::GLProfile;

use crate::render_frame::FrameRenderEngine;

const TRANSPORT_WIDTH: u32 = 640;
const TRANSPORT_HEIGHT: u32 = 360;
const TRANSPORT_FRAME_INTERVAL: Duration = Duration::from_millis(100);
const TRANSPORT_FILENAME: &str = "screenshaver-lock-test.rgba";
const TRANSPORT_TEMP_FILENAME: &str = ".screenshaver-lock-test.rgba.tmp";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Temporary GNOME lock-screen presentation host.
///
/// This is deliberately a proof-of-integration transport. The renderer remains
/// Screenshaver's existing `FrameRenderEngine`; completed RGBA frames are read
/// back from a hidden SDL/OpenGL surface and atomically published in
/// `$XDG_RUNTIME_DIR` for the GNOME Shell extension that was proven during the
/// external-frame test.
///
/// The runtime file transport is not intended to be the final production IPC
/// mechanism. Once real Screenshaver shader output is proven behind GNOME's
/// secure lock UI, this transport can be replaced without changing the shared
/// render engine.
pub(crate) struct GnomeLockPresenter {
    presenter_running: Arc<AtomicBool>,
    render_thread: Option<JoinHandle<Result<(), String>>>,
}

impl GnomeLockPresenter {
    pub(crate) fn start(
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
        let presenter_running = Arc::new(AtomicBool::new(true));
        let thread_running = Arc::clone(&presenter_running);
        let thread_logfile = logfile.to_path_buf();
        let (startup_sender, startup_receiver) = mpsc::channel::<Result<(), String>>();

        let render_thread = thread::Builder::new()
            .name("screenshaver-gnome-lock-renderer".to_string())
            .spawn(move || {
                let mut producer = match GnomeLockFrameProducer::new(
                    &thread_logfile,
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
                ) {
                    Ok(producer) => producer,
                    Err(error) => {
                        let _ = startup_sender.send(Err(error.clone()));
                        return Err(error);
                    }
                };

                if startup_sender.send(Ok(())).is_err() {
                    return Err(
                        "GNOME lock presenter startup receiver disappeared"
                            .to_string(),
                    );
                }

                producer.run(thread_running.as_ref())
            })
            .map_err(|error| {
                format!("Unable to start GNOME lock render thread: {error}")
            })?;

        match startup_receiver.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(())) => {
                log_information(
                    logfile,
                    "[LOCK] GNOME shader presentation backend initialized",
                );

                Ok(Self {
                    presenter_running,
                    render_thread: Some(render_thread),
                })
            }

            Ok(Err(error)) => {
                presenter_running.store(false, Ordering::SeqCst);
                let _ = render_thread.join();
                Err(error)
            }

            Err(error) => {
                presenter_running.store(false, Ordering::SeqCst);
                let _ = render_thread.join();

                Err(format!(
                    "Timed out waiting for GNOME lock presenter startup: {error}"
                ))
            }
        }
    }

    pub(crate) fn stop_and_join(mut self) -> Result<(), String> {
        self.presenter_running.store(false, Ordering::SeqCst);

        let Some(render_thread) = self.render_thread.take() else {
            return Ok(());
        };

        match render_thread.join() {
            Ok(result) => result,
            Err(_) => Err("GNOME lock render thread panicked".to_string()),
        }
    }
}

impl Drop for GnomeLockPresenter {
    fn drop(&mut self) {
        self.presenter_running.store(false, Ordering::SeqCst);
    }
}

struct GnomeLockFrameProducer {
    logfile: PathBuf,

    // Keep the render engine before its OpenGL context/window so engine-owned
    // GL resources are dropped while the context is still alive.
    engine: FrameRenderEngine,
    _gl_context: sdl2::video::GLContext,
    window: sdl2::video::Window,
    _sdl: sdl2::Sdl,

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
            "[LOCK] Initializing GNOME hidden shader presentation surface",
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

        let sdl = sdl2::init()
            .map_err(|error| format!("Failed to initialize SDL for GNOME lock presentation: {error}"))?;

        let video = sdl
            .video()
            .map_err(|error| format!("Failed to initialize SDL video for GNOME lock presentation: {error}"))?;

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
                format!("Failed to create hidden GNOME lock renderer window: {error}")
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
            _sdl: sdl,
            frame_path,
            temp_path,
            readback: vec![0; frame_bytes],
            top_down_rgba: vec![0; frame_bytes],
            last_publish: Instant::now() - TRANSPORT_FRAME_INTERVAL,
            published_frames: 0,
        })
    }

    fn run(&mut self, presenter_running: &AtomicBool) -> Result<(), String> {
        log_information(
            &self.logfile,
            "[LOCK] GNOME shader presentation render loop started",
        );

        while presenter_running.load(Ordering::SeqCst) {
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

        let _ = fs::remove_file(&self.temp_path);
        let _ = fs::remove_file(&self.frame_path);

        log_information(
            &self.logfile,
            "[LOCK] GNOME shader presentation render loop stopped",
        );

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
}

fn log_information(logfile: &Path, message: &str) {
    crate::logger::information(logfile, message);
}
