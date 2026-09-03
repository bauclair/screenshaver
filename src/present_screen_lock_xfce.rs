use std::env;
use std::fs;
use std::mem::MaybeUninit;
use std::path::Path;
use std::time::{Duration, Instant};

use x11::glx;
use x11::xlib;

const XSCREENSAVER_WINDOW_ENV: &str = "XSCREENSAVER_WINDOW";
const XFCE_AUTH_DIALOG_EXECUTABLE: &str = "/usr/libexec/xfce4-screensaver-dialog";
const XFCE_AUTH_DIALOG_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Returns the X11 window supplied by xfce4-screensaver for an external
/// screensaver child.
///
/// xfce4-screensaver exports XSCREENSAVER_WINDOW before launching the
/// configured screensaver program. Screenshaver must render into this window;
/// it must not create or manage the secure lock window itself.
pub(crate) fn detect_presentation_window(
    logfile: &Path,
) -> Result<u64, String> {
    let raw_window =
        env::var(XSCREENSAVER_WINDOW_ENV)
            .map_err(|_| {
                format!(
                    "{} is not set; Screenshaver was not launched by an \
                     XScreenSaver-compatible host",
                    XSCREENSAVER_WINDOW_ENV,
                )
            })?;

    let raw_window =
        raw_window.trim();

    if raw_window.is_empty() {
        return Err(
            format!(
                "{} is empty",
                XSCREENSAVER_WINDOW_ENV,
            )
        );
    }

    let window =
        if let Some(hexadecimal) =
            raw_window
                .strip_prefix("0x")
                .or_else(|| raw_window.strip_prefix("0X"))
        {
            u64::from_str_radix(
                hexadecimal,
                16,
            )
            .map_err(|error| {
                format!(
                    "Unable to parse {} value '{}': {}",
                    XSCREENSAVER_WINDOW_ENV,
                    raw_window,
                    error,
                )
            })?
        } else {
            raw_window
                .parse::<u64>()
                .map_err(|error| {
                    format!(
                        "Unable to parse {} value '{}': {}",
                        XSCREENSAVER_WINDOW_ENV,
                        raw_window,
                        error,
                    )
                })?
        };

    if window == 0 {
        return Err(
            format!(
                "{} contains an invalid zero X11 window ID",
                XSCREENSAVER_WINDOW_ENV,
            )
        );
    }

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE lock presentation window detected: 0x{:X}",
            window,
        ),
    );

    verify_presentation_window(
        logfile,
        window,
    )?;

    Ok(
        window
    )
}


fn verify_presentation_window(
    logfile: &Path,
    window: u64,
) -> Result<(), String> {
    crate::logger::information(
        logfile,
        "[LOCK] XFCE OpenGL presentation: opening X11 display",
    );

    let connection =
        crate::x11_connection::X11Connection::connect()
            .map_err(|error| {
                format!(
                    "Unable to connect to the X11 display while verifying XFCE presentation window 0x{:X}: {}",
                    window,
                    error,
                )
            })?;

    let display = connection.display();
    let x11_window = window as xlib::Window;
    let mut attributes =
        MaybeUninit::<xlib::XWindowAttributes>::uninit();

    let status =
        unsafe {
            xlib::XGetWindowAttributes(
                display,
                x11_window,
                attributes.as_mut_ptr(),
            )
        };

    if status == 0 {
        return Err(
            format!(
                "XGetWindowAttributes failed for XFCE presentation window 0x{:X}",
                window,
            )
        );
    }

    let attributes =
        unsafe {
            attributes.assume_init()
        };

    if attributes.width <= 0 || attributes.height <= 0 {
        return Err(
            format!(
                "XFCE presentation window 0x{:X} has invalid geometry {}x{}",
                window,
                attributes.width,
                attributes.height,
            )
        );
    }

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE lock presentation window verified: 0x{:X}, geometry={}x{}, depth={}, map_state={}",
            window,
            attributes.width,
            attributes.height,
            attributes.depth,
            attributes.map_state,
        ),
    );

    run_opengl_clear_test(
        logfile,
        &connection,
        x11_window,
        attributes.width,
        attributes.height,
    )
}


fn run_opengl_clear_test(
    logfile: &Path,
    connection: &crate::x11_connection::X11Connection,
    window: xlib::Window,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let display = connection.display();
    let screen = connection.screen();

    crate::logger::information(
        logfile,
        "[LOCK] XFCE shader presentation: loading Screenshaver configuration",
    );

    let config_path =
        crate::locate_paths::config_path();

    let config_result =
        crate::load_config::load_config(
            &config_path
        )
        .map_err(|error| {
            format!(
                "Unable to load Screenshaver configuration for XFCE lock presentation: {}",
                error,
            )
        })?;

    let cfg =
        config_result.config;

    let parsed_mode =
        crate::parse_mode::parse_mode(
            &cfg.mode
        );

    let shader_mode =
        match cfg.mode
            .split(':')
            .next()
            .unwrap_or("single")
        {
            "single" => {
                crate::manage_shader::ShaderMode::Single(
                    parsed_mode.argument.clone()
                )
            }

            "random" => {
                crate::manage_shader::ShaderMode::Random
            }

            "ordered" => {
                crate::manage_shader::ShaderMode::Ordered
            }

            _ => {
                crate::manage_shader::ShaderMode::Single(
                    parsed_mode.argument.clone()
                )
            }
        };

    let shader_interval =
        match cfg.mode
            .split(':')
            .next()
            .unwrap_or("single")
        {
            "single" => 0,

            "random" | "ordered" => {
                let interval_source =
                    cfg.mode
                        .split(':')
                        .nth(1)
                        .unwrap_or("60");

                crate::parse_interval::parse_interval(
                    interval_source
                )
                .seconds
            }

            _ => 0,
        };

    let shader_manager =
        crate::manage_shader::ShaderManager::new(
            shader_mode
        );

    let audio_backend =
        crate::audio_backend::create_backend()
            .ok();

    let audio_bands =
        audio_backend
            .as_ref()
            .map(
                |backend| {
                    backend.shared_bands()
                }
            );

    crate::logger::information(
        logfile,
        "[LOCK] XFCE shader presentation: choosing GLX framebuffer configuration",
    );

    let framebuffer_config =
        crate::glx_context::GlxFramebufferConfig::choose(
            display,
            screen,
        )
        .map_err(|error| {
            format!(
                "Unable to choose GLX framebuffer configuration for XFCE lock presentation: {}",
                error,
            )
        })?;

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE shader presentation: selected GLX visual 0x{:X}",
            framebuffer_config.visual_info().visualid,
        ),
    );

    let context =
        crate::glx_context::GlxContext::create(
            display,
            &framebuffer_config,
        )
        .map_err(|error| {
            format!(
                "Unable to create GLX context for XFCE lock presentation: {}",
                error,
            )
        })?;

    crate::logger::information(
        logfile,
        "[LOCK] XFCE shader presentation: making context current on supplied window",
    );

    if let Err(error) =
        context.make_current(
            display,
            window,
        )
    {
        context.destroy(
            display
        );

        return Err(
            format!(
                "Unable to make GLX context current on XFCE presentation window 0x{:X}: {}",
                window,
                error,
            )
        );
    }

    crate::logger::information(
        logfile,
        "[LOCK] XFCE shader presentation: GLX context is current",
    );

    gl::load_with(
        |symbol| {
            let symbol =
                std::ffi::CString::new(symbol)
                    .expect("OpenGL symbol name contained an interior NUL");

            unsafe {
                glx::glXGetProcAddress(
                    symbol.as_ptr() as *const u8
                )
                .map_or(
                    std::ptr::null(),
                    |function| {
                        function as *const () as *const std::ffi::c_void
                    },
                )
            }
        }
    );

    crate::logger::information(
        logfile,
        "[LOCK] XFCE shader presentation: constructing FrameRenderEngine",
    );

    let mut engine =
        match crate::render_frame::FrameRenderEngine::new(
            shader_manager,
            shader_interval,
            cfg.screensaver_speed_policy.clone(),
            cfg.global_rendered_fps,
            cfg.screensaver_fps_policy_entries.clone(),
            cfg.texture_policy.clone(),
            cfg.screensaver_postprocess_policy.clone(),
            audio_bands,
            cfg.subtitles,
            cfg.subtitle_placement,
            width as u32,
            height as u32,
        ) {
            Ok(engine) => engine,

            Err(error) => {
                let _ =
                    crate::glx_context::GlxContext::release_current(
                        display
                    );

                context.destroy(
                    display
                );

                return Err(
                    format!(
                        "Unable to construct FrameRenderEngine for XFCE lock presentation: {}",
                        error,
                    )
                );
            }
        };

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE shader presentation initialized: window=0x{:X}, geometry={}x{}",
            window,
            width,
            height,
        ),
    );

    let mut authentication_dialog_state =
        match XfceAuthenticationDialogState::new(
            logfile,
        ) {
            Ok(state) => Some(state),

            Err(error) => {
                crate::logger::warning(
                    logfile,
                    &format!(
                        "[LOCK] XFCE authentication-dialog process reconnaissance unavailable; shader presentation will continue: {}",
                        error,
                    ),
                );

                None
            }
        };

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE shader presentation started: window=0x{:X}, geometry={}x{}",
            window,
            width,
            height,
        ),
    );

    let mut rendered_frames =
        0u64;

    loop {
        let _ =
            engine.render_frame(
                width as u32,
                height as u32,
            );

        unsafe {
            glx::glXSwapBuffers(
                display,
                window,
            );
        }

        if let Some(state) =
            authentication_dialog_state.as_mut()
        {
            state.poll_if_due(
                logfile,
            );
        }

        rendered_frames =
            rendered_frames.saturating_add(
                1
            );

        if rendered_frames % 300 == 0 {
            crate::logger::information(
                logfile,
                &format!(
                    "[LOCK] XFCE shader presentation frames displayed: {}",
                    rendered_frames,
                ),
            );
        }

        engine.limit_fps();
    }
}



struct XfceAuthenticationDialogState {
    visible: bool,
    next_poll: Instant,
}


impl XfceAuthenticationDialogState {
    fn new(
        logfile: &Path,
    ) -> Result<Self, String> {
        let visible =
            xfce_authentication_dialog_running()?;

        log_authentication_dialog_state(
            logfile,
            visible,
        );

        Ok(
            Self {
                visible,
                next_poll: Instant::now()
                    + XFCE_AUTH_DIALOG_POLL_INTERVAL,
            }
        )
    }


    fn poll_if_due(
        &mut self,
        logfile: &Path,
    ) {
        let now =
            Instant::now();

        if now < self.next_poll {
            return;
        }

        self.next_poll =
            now + XFCE_AUTH_DIALOG_POLL_INTERVAL;

        let visible =
            match xfce_authentication_dialog_running() {
                Ok(visible) => visible,

                Err(error) => {
                    crate::logger::warning(
                        logfile,
                        &format!(
                            "[LOCK] XFCE authentication-dialog process poll failed: {}",
                            error,
                        ),
                    );

                    return;
                }
            };

        if visible == self.visible {
            return;
        }

        self.visible =
            visible;

        log_authentication_dialog_state(
            logfile,
            visible,
        );
    }
}


fn log_authentication_dialog_state(
    logfile: &Path,
    visible: bool,
) {
    let state =
        if visible {
            "visible"
        } else {
            "hidden"
        };

    crate::logger::information(
        logfile,
        &format!(
            "[LOCK] XFCE authentication dialog state: {}",
            state,
        ),
    );
}


fn xfce_authentication_dialog_running() -> Result<bool, String> {
    let entries =
        fs::read_dir(
            "/proc"
        )
        .map_err(|error| {
            format!(
                "Unable to read /proc while checking for {}: {}",
                XFCE_AUTH_DIALOG_EXECUTABLE,
                error,
            )
        })?;

    for entry in entries {
        let entry =
            match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

        let file_name =
            entry.file_name();

        let Some(pid) =
            file_name
                .to_str()
                .filter(|name| {
                    !name.is_empty()
                        && name.bytes().all(|byte| byte.is_ascii_digit())
                })
        else {
            continue;
        };

        let cmdline_path =
            Path::new("/proc")
                .join(pid)
                .join("cmdline");

        let cmdline =
            match fs::read(
                &cmdline_path
            ) {
                Ok(cmdline) => cmdline,
                Err(_) => continue,
            };

        let executable =
            cmdline
                .split(|byte| *byte == 0)
                .next()
                .unwrap_or(&[]);

        if executable
            == XFCE_AUTH_DIALOG_EXECUTABLE.as_bytes()
        {
            return Ok(
                true
            );
        }
    }

    Ok(
        false
    )
}
