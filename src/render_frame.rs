use std::path::PathBuf;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::video::{GLContext, GLProfile, Window};

#[path = "render_frame_engine.rs"]
mod render_frame_engine;

pub(crate) use render_frame_engine::{
    FrameRenderEngine,
    FrameRenderEvent,
    FrameRenderEvents,
    FrameRenderMetadata,
};

// Explicit opt-in: SCREENSHAVER_AMBIENT_SAMPLE_DIAGNOSTIC=1 screenshaver
// This test never opens an OpenRGB connection or changes device lighting.
struct AmbientSamplingDiagnostic {
    sampler: crate::manage_ambient_lighting::FramebufferSampler,
    captured: u64,
    total_us: u128,
    maximum_us: u128,
    last_report: Instant,
    disabled: bool,
    live_dropped: u64,
    live_hardware: Option<crate::manage_openrgb_session::ShaderLightingSession>,
}

impl AmbientSamplingDiagnostic {
    fn new() -> Result<Self, String> {
        Ok(Self {
            sampler: crate::manage_ambient_lighting::FramebufferSampler::new(
                22, 12, Duration::from_millis(50),
            )?,
            captured: 0,
            total_us: 0,
            maximum_us: 0,
            last_report: Instant::now(),
            disabled: false,
            live_dropped: 0,
            live_hardware: None,
        })
    }

    fn stop_live_hardware(&mut self) {
        if let Some(session) = self.live_hardware.take() {
            match session.stop() {
                Ok((submitted, stats)) => log_information(&format!(
                    "[AMBIENT_OPENRGB] Live session finished; submitted={submitted} transmitted={} queue_dropped={} original lighting restored",
                    stats.transmitted,
                    self.live_dropped,
                )),
                Err(error) => log_warning(&format!("[AMBIENT_OPENRGB] {error}")),
            }
        }
    }

    fn observe(&mut self, framebuffer: u32, width: u32, height: u32) {
        if self.disabled || framebuffer != 0 || width == 0 || height == 0 { return; }
        let source = crate::manage_ambient_lighting::FrameSource {
            framebuffer, width, height,
        };
        match self.sampler.sample(source) {
            Ok(Some(frame)) => {
                let elapsed = frame.readback_time.as_micros();
                self.captured += 1;
                if let Some(ref mut hardware) = self.live_hardware {
                    match hardware.submit(&frame) {
                        Ok(false) => self.live_dropped += 1,
                        Ok(true) => {},
                        Err(error) => {
                            log_warning(&format!("[AMBIENT_OPENRGB] Live update failed: {error}"));
                            self.stop_live_hardware();
                        }
                    }
                }
                self.total_us += elapsed;
                self.maximum_us = self.maximum_us.max(elapsed);
                if self.last_report.elapsed() >= Duration::from_secs(5) {
                    let rgb = crate::manage_ambient_lighting::representative_color(&frame.rgb);
                    log_information(&format!(
                        "[AMBIENT_DIAGNOSTIC] samples={} grid={}x{} avg_readback_us={} max_readback_us={} representative_rgb=#{:02X}{:02X}{:02X}",
                        self.captured, frame.width, frame.height,
                        self.total_us / self.captured as u128,
                        self.maximum_us, rgb[0], rgb[1], rgb[2],
                    ));
                    self.last_report = Instant::now();
                }
            }
            Ok(None) => {}
            Err(error) => {
                self.disabled = true;
                log_warning(&format!(
                    "[AMBIENT_DIAGNOSTIC] Sampling disabled after error: {error}"
                ));
            }
        }
    }
}

const INPUT_STARTUP_GRACE: Duration = Duration::from_millis(750);
const MOUSE_MOTION_EXIT_THRESHOLD: i32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreensaverRunOutcome {
    Exit,
    EditCurrentShader {
        shader_path: PathBuf,
        policy_id: i64,
    },
}


pub struct FrameRenderer {
    // Keep the engine first so its OpenGL resources are released before the
    // context and window are destroyed.
    engine: FrameRenderEngine,
    ambient_diagnostic: Option<Rc<RefCell<AmbientSamplingDiagnostic>>>,
    event_pump: sdl2::EventPump,
    renderer_started: Instant,
    _gl_context: GLContext,
    window: Window,
}

impl FrameRenderer {
    pub fn new(
        sdl: &sdl2::Sdl,
        shader_manager: crate::manage_shader::ShaderManager,
        shader_interval: u64,
        animation_speed_policy:
            crate::load_config::AnimationSpeedPolicy,
        global_rendered_fps: u32,
        fps_policy_entries:
            Vec<
                crate::load_config::FpsPolicyEntry
            >,
        texture_policy:
            crate::load_config::TexturePolicy,
        postprocess_policy:
            crate::load_config::PostprocessPolicy,
        audio_bands:
            Option<crate::audio_backend::SharedAudioBands>,
        subtitles: bool,
        subtitle_placement:
            crate::parse_subtitle_placement::SubtitlePlacement,
    ) -> Result<Self, String> {
        log_information("[RENDER] Initializing frame renderer");

        let video = sdl.video()?;

        let event_pump = sdl
            .event_pump()
            .map_err(
                |error| {
                    format!(
                        "Failed to create SDL event pump: {error}"
                    )
                }
            )?;

        {
            let gl_attr = video.gl_attr();

            gl_attr.set_context_profile(
                GLProfile::Core
            );

            gl_attr.set_context_version(
                crate::define_constants::GL_MAJOR,
                crate::define_constants::GL_MINOR,
            );
        }

        let mut window = video
            .window(
                crate::define_constants::WINDOW_TITLE,
                0,
                0,
            )
            .fullscreen_desktop()
            .borderless()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Failed to create renderer window: {error}"
                    )
                }
            )?;

        let (window_width, window_height) =
            window.size();

        log_information(
            &format!(
                "[RENDER] Window created: {window_width}x{window_height}"
            )
        );

        let gl_context = window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Failed to create OpenGL context: {error}"
                    )
                }
            )?;

        window.raise();

        let _ = window.set_fullscreen(
            sdl2::video::FullscreenType::Desktop
        );

        gl::load_with(
            |symbol| {
                video.gl_get_proc_address(
                    symbol
                ) as *const _
            }
        );

        let _ = video.gl_set_swap_interval(
            0
        );

        let mut engine =
            FrameRenderEngine::new(
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
                window_width,
                window_height,
            )?;

        let live_opt_in = std::env::var("SCREENSHAVER_AMBIENT_LIVE_CONTROLLER")
            .ok().and_then(|value| value.parse::<u32>().ok());
        let ambient_diagnostic = if live_opt_in.is_some() || std::env::var_os("SCREENSHAVER_AMBIENT_SAMPLE_DIAGNOSTIC")
            .is_some_and(|value| value == "1")
        {
            let diagnostic = Rc::new(RefCell::new(AmbientSamplingDiagnostic::new()?));
            if let Some(controller_id) = live_opt_in {
                let endpoint = std::env::var("SCREENSHAVER_AMBIENT_LIVE_ENDPOINT")
                    .unwrap_or_else(|_| "127.0.0.1:6742".to_string());
                match endpoint.parse::<std::net::SocketAddr>() {
                    Ok(address) => match crate::manage_openrgb_session::ShaderLightingSession::start(address, controller_id) {
                        Ok(session) => {
                            diagnostic.borrow_mut().live_hardware = Some(session);
                            log_information(&format!(
                                "[AMBIENT_OPENRGB] Guarded shader mapping active for controller {controller_id}; renderer-lifetime session"
                            ));
                        }
                        Err(error) => log_warning(&format!("[AMBIENT_OPENRGB] Live acquisition refused: {error}")),
                    },
                    Err(error) => log_warning(&format!("[AMBIENT_OPENRGB] Invalid endpoint: {error}")),
                }
            }
            let observer = Rc::clone(&diagnostic);
            engine.set_ambient_frame_hook(Some(Box::new(move |framebuffer, width, height| {
                observer.borrow_mut().observe(framebuffer, width, height);
            })));
            log_information("[AMBIENT_DIAGNOSTIC] Enabled: 22x12 RGB readback at <=20 Hz; hardware access only when explicitly enabled");
            Some(diagnostic)
        } else {
            None
        };

        Ok(
            Self {
                engine,
                ambient_diagnostic,
                event_pump,
                renderer_started:
                    Instant::now(),
                _gl_context:
                    gl_context,
                window,
            }
        )
    }


    pub fn run(
        &mut self,
        running: &AtomicBool,
        wallpaper_control: &crate::manage_wallpaper_runtime::WallpaperRuntimeControl,
    ) -> ScreensaverRunOutcome {
        log_information(
            "[RENDER] Entering renderer-owned event loop"
        );

        let mut first_frame_presented =
            false;

        let outcome = loop {
            if !running.load(Ordering::SeqCst) {
                break ScreensaverRunOutcome::Exit;
            }

            match self.pump_events() {
                Some(outcome) => {
                    break outcome;
                }
                None => {}
            }

            self.render_frame();

            if !first_frame_presented {
                first_frame_presented =
                    true;

                wallpaper_control.request_pause_after_first_frame(
                    running
                );
            }
        };

        // Keep wallpaper rendering paused while the active screensaver is
        // handed to the policy editor. A replacement screensaver renderer
        // will retain that pause until the user finally disengages it.
        // Restore the device before the wallpaper renderer can resume.
        if let Some(diagnostic) = self.ambient_diagnostic.as_ref() {
            diagnostic.borrow_mut().stop_live_hardware();
        }
        if outcome == ScreensaverRunOutcome::Exit {
            wallpaper_control.resume_and_wait_for_frame(
                running
            );
        }

        log_information(
            "[RENDER] Leaving renderer-owned event loop"
        );

        outcome
    }


    pub fn render_frame(
        &mut self,
    ) {
        let (
            width,
            height,
        ) =
            self.window.drawable_size();

        let _ = self.engine.render_frame(
            width,
            height,
        );

        self.window.gl_swap_window();
        self.engine.limit_fps();
    }


    fn pump_events(
        &mut self,
    ) -> Option<ScreensaverRunOutcome> {
        let mouse_motion_enabled =
            self.renderer_started.elapsed()
                >= INPUT_STARTUP_GRACE;

        for event in
            self.event_pump.poll_iter()
        {
            match event {
                Event::Quit {
                    ..
                } => {
                    log_information(
                        "[RENDER] SDL quit event received"
                    );

                    return Some(ScreensaverRunOutcome::Exit);
                }

                Event::Window {
                    win_event,
                    ..
                } => {
                    log_debug(
                        &format!(
                            "[RENDER] SDL window event: {win_event:?}"
                        )
                    );
                }

                Event::KeyDown {
                    keycode: Some(Keycode::E),
                    keymod,
                    repeat: false,
                    ..
                } if edit_shortcut_modifiers_allowed(keymod) => {
                    let metadata =
                        self.engine.current_metadata();

                    if let Some(shader_path) =
                        metadata.shader_path
                    {
                        log_information(
                            &format!(
                                "[RENDER] E pressed: editing active screensaver policy '{}' (policy_id={})",
                                metadata.policy_name,
                                metadata.policy_id,
                            )
                        );

                        return Some(
                            ScreensaverRunOutcome::EditCurrentShader {
                                shader_path,
                                policy_id:
                                    metadata.policy_id,
                            }
                        );
                    }

                    log_warning(
                        "[RENDER] E pressed, but the active shader has no editable source path; exiting normally"
                    );

                    return Some(ScreensaverRunOutcome::Exit);
                }

                Event::KeyDown {
                    ..
                } => {
                    log_information(
                        "[RENDER] SDL keydown event: exiting"
                    );

                    return Some(ScreensaverRunOutcome::Exit);
                }

                Event::MouseButtonDown {
                    ..
                } => {
                    log_information(
                        "[RENDER] SDL mouse button event: exiting"
                    );

                    return Some(ScreensaverRunOutcome::Exit);
                }

                Event::MouseWheel {
                    ..
                } => {
                    log_information(
                        "[RENDER] SDL mouse wheel event: exiting"
                    );

                    return Some(ScreensaverRunOutcome::Exit);
                }

                Event::MouseMotion {
                    xrel,
                    yrel,
                    ..
                } => {
                    if !mouse_motion_enabled {
                        log_debug(
                            "[RENDER] Ignoring startup mouse motion"
                        );

                        continue;
                    }

                    if xrel.abs()
                        >= MOUSE_MOTION_EXIT_THRESHOLD
                        || yrel.abs()
                            >= MOUSE_MOTION_EXIT_THRESHOLD
                    {
                        log_information(
                            &format!(
                                "[RENDER] SDL mouse motion event: exiting (xrel={xrel}, yrel={yrel})"
                            )
                        );

                        return Some(ScreensaverRunOutcome::Exit);
                    }
                }

                _ => {}
            }
        }

        None
    }


}


// Drop runs while the SDL GL context and window fields are still alive.
// The callback is detached first so the sampler has a single owner during cleanup.
impl Drop for FrameRenderer {
    fn drop(&mut self) {
        self.engine.set_ambient_frame_hook(None);
        if let Some(diagnostic) = self.ambient_diagnostic.take() {
            let mut diagnostic = diagnostic.borrow_mut();
            log_information(&format!(
                "[AMBIENT_DIAGNOSTIC] Final samples={} avg_readback_us={} max_readback_us={}",
                diagnostic.captured,
                if diagnostic.captured == 0 { 0 } else { diagnostic.total_us / diagnostic.captured as u128 },
                diagnostic.maximum_us,
            ));
            diagnostic.stop_live_hardware();
            unsafe { diagnostic.sampler.destroy(); }
        }
    }
}

fn edit_shortcut_modifiers_allowed(
    keymod: Mod,
) -> bool {
    let allowed_lock_modifiers =
        Mod::NUMMOD | Mod::CAPSMOD;

    (keymod & !allowed_lock_modifiers)
        == Mod::NOMOD
}


fn log_debug(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::debug(
        &logfile,
        message,
    );
}


fn log_information(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::information(
        &logfile,
        message,
    );
}


fn log_warning(
    message: &str,
) {
    let logfile: PathBuf =
        crate::locate_paths::runtime_log_path();

    crate::logger::warning(
        &logfile,
        message,
    );
}

