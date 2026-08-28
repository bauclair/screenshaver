use std::path::PathBuf;
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

const INPUT_STARTUP_GRACE: Duration = Duration::from_millis(750);
const MOUSE_MOTION_EXIT_THRESHOLD: i32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreensaverRunOutcome {
    Exit,
    EditCurrentShader(PathBuf),
}


pub struct FrameRenderer {
    // Keep the engine first so its OpenGL resources are released before the
    // context and window are destroyed.
    engine: FrameRenderEngine,
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

        let engine =
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

        Ok(
            Self {
                engine,
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
                    if let Some(shader_path) =
                        self.engine.current_metadata().shader_path
                    {
                        log_information(
                            "[RENDER] E pressed: editing active screensaver shader"
                        );

                        return Some(
                            ScreensaverRunOutcome::EditCurrentShader(
                                shader_path
                            )
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

