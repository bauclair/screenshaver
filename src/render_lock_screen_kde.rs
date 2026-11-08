use std::ffi::{CString, c_char, c_void};
use std::ptr;
use std::sync::{LazyLock, Mutex};

pub type GlProcLoader = unsafe extern "C" fn(*const c_char) -> *const c_void;

pub struct KdeFrameRenderer {
    // KDE runs this renderer inside the KScreenLocker greeter process.  That
    // process gets its own demand-driven AudioRuntime; FrameRenderEngine tells
    // it when the active policy needs Audio Bloom.
    engine: crate::render_frame_engine::FrameRenderEngine,
}

static LAST_ERROR: LazyLock<Mutex<CString>> =
    LazyLock::new(|| Mutex::new(CString::new("").expect("empty CString")));

fn set_last_error(message: impl Into<String>) {
    let sanitized = message.into().replace('\0', " ");

    if let Ok(mut error) = LAST_ERROR.lock() {
        *error = CString::new(sanitized)
            .unwrap_or_else(|_| CString::new("Screenshaver KDE renderer error").expect("static CString"));
    }
}

fn clear_last_error() {
    set_last_error("");
}

fn last_error_ptr() -> *const c_char {
    let Ok(error) = LAST_ERROR.lock() else {
        return b"Screenshaver KDE renderer error\0".as_ptr().cast();
    };

    error.as_ptr()
}

fn load_gl(loader: GlProcLoader) -> Result<(), String> {
    gl::load_with(|symbol| {
        let Ok(symbol) = CString::new(symbol) else {
            return ptr::null();
        };

        unsafe { loader(symbol.as_ptr()) }
    });

    if !gl::CreateShader::is_loaded()
        || !gl::CreateProgram::is_loaded()
        || !gl::GenVertexArrays::is_loaded()
        || !gl::GenFramebuffers::is_loaded()
        || !gl::BindFramebuffer::is_loaded()
        || !gl::DrawArrays::is_loaded()
    {
        return Err(
            "Required OpenGL 3.3 entry points were not supplied by the current Qt context"
                .to_string(),
        );
    }

    Ok(())
}

fn screensaver_mode(
    cfg: &crate::load_config::Config,
) -> crate::manage_shader::ShaderMode {
    let parsed_mode = crate::parse_mode::parse_mode(&cfg.mode);

    match cfg.mode
        .split(':')
        .next()
        .unwrap_or("single")
    {
        "single" => crate::manage_shader::ShaderMode::Single(parsed_mode.argument),
        "random" => crate::manage_shader::ShaderMode::Random,
        "ordered" => crate::manage_shader::ShaderMode::Ordered,
        _ => crate::manage_shader::ShaderMode::Single(parsed_mode.argument),
    }
}

fn screensaver_interval(
    cfg: &crate::load_config::Config,
) -> u64 {
    match cfg.mode
        .split(':')
        .next()
        .unwrap_or("single")
    {
        "random" | "ordered" => {
            let interval_source = cfg.mode
                .split(':')
                .nth(1)
                .unwrap_or("60");

            crate::parse_interval::parse_interval(interval_source).seconds
        }
        _ => 0,
    }
}

unsafe fn create_renderer(
    loader: Option<GlProcLoader>,
    width: i32,
    height: i32,
) -> Result<*mut KdeFrameRenderer, String> {
    let loader = loader.ok_or_else(|| "Qt OpenGL procedure loader was null".to_string())?;

    if width <= 0 || height <= 0 {
        return Err(format!("Invalid KDE render size: {width}x{height}"));
    }

    load_gl(loader)?;

    let config_path = crate::locate_paths::config_path();
    let config_result = crate::load_config::load_config(&config_path)
        .map_err(|error| {
            format!(
                "Unable to load Screenshaver configuration {}: {error}",
                config_path.display(),
            )
        })?;

    let cfg = config_result.config;
    let shader_mode = screensaver_mode(&cfg);
    let shader_interval = screensaver_interval(&cfg);
    let shader_manager = crate::manage_shader::ShaderManager::new(shader_mode);

    // KScreenLocker loads this Rust renderer into the greeter process, so it
    // cannot share the resident Screenshaver process's in-memory bands.  The
    // process-local handle remains zero until FrameRenderEngine activates
    // Audio Bloom capture for an Audio policy.
    let audio_bands =
        Some(
            crate::audio_backend::shared_audio_bands()
        );

    // Preserve the configured Screenshaver subtitle/descriptive-pill policy
    // inside KScreenLocker. FPS Warning/CRITICAL overlays remain available
    // independently through FrameRenderEngine when subtitles are disabled.
    let engine = crate::render_frame_engine::FrameRenderEngine::new(
        shader_manager,
        shader_interval,
        cfg.screensaver_speed_policy,
        cfg.global_rendered_fps,
        cfg.screensaver_fps_policy_entries,
        cfg.texture_policy,
        cfg.screensaver_postprocess_policy,
        audio_bands,
        cfg.subtitles,
        cfg.subtitle_placement,
        width as u32,
        height as u32,
    )?;

    Ok(Box::into_raw(Box::new(KdeFrameRenderer {
        engine,
    })))
}

// KDE Plasma wallpaper mode: policy discovery stays inside the renderer library.
// Do not pull the application's wallpaper backend/supervisor into this cdylib.
fn wallpaper_selection(
    configured_mode: &str,
) -> Result<(crate::manage_shader::ShaderMode, u64), String> {
    use crate::parse_mode::ModeType;
    use crate::manage_shader::ShaderMode;

    let parsed = crate::parse_mode::parse_mode(configured_mode);
    match parsed.mode {
        ModeType::Single => Ok((ShaderMode::Single(parsed.argument), 0)),
        ModeType::Random => Ok((
            ShaderMode::Random,
            crate::parse_interval::parse_interval(&parsed.argument).seconds,
        )),
        ModeType::Ordered => Ok((
            ShaderMode::Ordered,
            crate::parse_interval::parse_interval(&parsed.argument).seconds,
        )),
        ModeType::Playlist => {
            let playlist_id = parsed.argument.parse::<i64>()
                .map_err(|error| format!("Invalid KDE wallpaper playlist ID: {error}"))?;
            if playlist_id <= 0 {
                return Err("KDE wallpaper playlist ID must be positive".to_string());
            }
            let seconds = configured_mode.split(':').nth(2)
                .ok_or_else(|| "KDE wallpaper playlist interval is missing".to_string())?
                .parse::<u64>()
                .map_err(|error| format!("Invalid KDE wallpaper playlist interval: {error}"))?;
            if seconds == 0 {
                return Err("KDE wallpaper playlist interval must be positive".to_string());
            }
            Ok((ShaderMode::Playlist(playlist_id), seconds))
        }
        ModeType::Invalid => Err(format!(
            "Unsupported KDE wallpaper mode: {configured_mode}"
        )),
    }
}

unsafe fn create_wallpaper_renderer(
    loader: Option<GlProcLoader>,
    width: i32,
    height: i32,
) -> Result<*mut KdeFrameRenderer, String> {
    let loader = loader.ok_or_else(|| "Qt OpenGL procedure loader was null".to_string())?;
    if width <= 0 || height <= 0 {
        return Err(format!("Invalid KDE wallpaper render size: {width}x{height}"));
    }
    load_gl(loader)?;

    let config_path = crate::locate_paths::config_path();
    let cfg = crate::load_config::load_config(&config_path)
        .map_err(|error| format!("Unable to load KDE wallpaper configuration: {error}"))?
        .config;
    if !cfg.wallpaper_enabled {
        return Err("Screenshaver wallpaper is disabled in configuration".to_string());
    }

    let (shader_mode, mut shader_interval) = wallpaper_selection(&cfg.wallpaper_mode)?;
    let managed_directory = crate::locate_paths::shader_dir();
    let mut entries = Vec::new();
    for policy in &cfg.wallpaper_policies {
        let source_path = policy.source_path.clone()
            .unwrap_or_else(|| managed_directory.join(&policy.shader));
        if !source_path.is_file() {
            continue;
        }
        entries.push(crate::manage_shader::ShaderEntry::with_policy_id_source_path(
            policy.policy_id,
            policy.shader.clone(),
            policy.policy_key.clone(),
            source_path,
        ));
    }
    if entries.is_empty() {
        return Err("No available KDE wallpaper shader policies".to_string());
    }
    if entries.len() == 1 {
        shader_interval = 0;
    }

    let shader_manager =
        crate::manage_shader::ShaderManager::from_shader_entries_for_target(
            shader_mode,
            entries,
            crate::manage_shader::OrderedTarget::Wallpaper,
        );
    let engine = crate::render_frame_engine::FrameRenderEngine::new_for_wallpaper(
        shader_manager,
        &managed_directory,
        shader_interval,
        cfg.wallpaper_speed_policy,
        cfg.wallpaper_fps_policy,
        cfg.wallpaper_texture_policy,
        cfg.wallpaper_postprocess_policy,
        Some(crate::audio_backend::shared_audio_bands()),
        false,
        cfg.subtitle_placement,
        None,
        width as u32,
        height as u32,
    )?;
    Ok(Box::into_raw(Box::new(KdeFrameRenderer { engine })))
}

unsafe fn render_frame(
    renderer: *mut KdeFrameRenderer,
    width: i32,
    height: i32,
) -> Result<(), String> {
    if renderer.is_null() {
        return Err("Screenshaver KDE renderer handle was null".to_string());
    }

    if width <= 0 || height <= 0 {
        return Ok(());
    }

    let renderer = unsafe { &mut *renderer };

    // QQuickWindow may render into a non-zero scene-graph framebuffer. Capture
    // the host target before FrameRenderEngine binds its own postprocess FBOs.
    let mut output_framebuffer = 0_i32;

    unsafe {
        gl::GetIntegerv(
            gl::DRAW_FRAMEBUFFER_BINDING,
            &mut output_framebuffer,
        );
    }

    let _events = renderer.engine.render_frame_to_framebuffer(
        width as u32,
        height as u32,
        output_framebuffer.max(0) as u32,
    );

    // Do not call FrameRenderEngine::limit_fps() from the Qt scene-graph render
    // thread. Qt owns presentation cadence; blocking that thread would stall
    // the lock-screen compositor. The engine still resolves and evaluates the
    // configured FPS policy for its normal performance accounting.

    Ok(())
}

unsafe fn destroy_renderer(renderer: *mut KdeFrameRenderer) {
    if renderer.is_null() {
        return;
    }

    // FrameRenderEngine::drop() deletes GL resources. The C++ render node calls
    // this only while the Qt OpenGL context is current.
    drop(unsafe { Box::from_raw(renderer) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn screenshaver_kde_gl_create(
    loader: Option<GlProcLoader>,
    width: i32,
    height: i32,
) -> *mut KdeFrameRenderer {
    clear_last_error();

    match unsafe { create_renderer(loader, width, height) } {
        Ok(renderer) => renderer,
        Err(error) => {
            set_last_error(error);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn screenshaver_kde_gl_create_for_mode(
    loader: Option<GlProcLoader>,
    width: i32,
    height: i32,
    mode: u32,
) -> *mut KdeFrameRenderer {
    clear_last_error();
    match mode {
        0 => unsafe { screenshaver_kde_gl_create(loader, width, height) },
        1 => match unsafe { create_wallpaper_renderer(loader, width, height) } {
            Ok(renderer) => renderer,
            Err(error) => {
                set_last_error(error);
                ptr::null_mut()
            }
        },
        _ => {
            set_last_error(format!("Unknown KDE renderer mode: {mode}"));
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn screenshaver_kde_gl_render(
    renderer: *mut KdeFrameRenderer,
    width: i32,
    height: i32,
) -> bool {
    match unsafe { render_frame(renderer, width, height) } {
        Ok(()) => true,
        Err(error) => {
            set_last_error(error);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn screenshaver_kde_gl_destroy(
    renderer: *mut KdeFrameRenderer,
) {
    unsafe { destroy_renderer(renderer) };
}

#[unsafe(no_mangle)]
pub extern "C" fn screenshaver_kde_gl_last_error() -> *const c_char {
    last_error_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn screenshaver_kde_gl_bridge_version() -> u32 {
    4
}
