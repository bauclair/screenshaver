use std::ffi::{CString, c_char, c_void};
use std::ptr;
use std::sync::{LazyLock, Mutex};

pub type GlProcLoader = unsafe extern "C" fn(*const c_char) -> *const c_void;

pub struct KdeFrameRenderer {
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

    // Phase 2 deliberately does not start a second audio-capture backend or
    // subtitle/TTF overlay inside KScreenLocker. The shader, texture, palette,
    // animation, FPS, and postprocess policies still come from Screenshaver.
    let engine = crate::render_frame_engine::FrameRenderEngine::new(
        shader_manager,
        shader_interval,
        cfg.screensaver_speed_policy,
        cfg.global_rendered_fps,
        cfg.screensaver_fps_policy_entries,
        cfg.texture_policy,
        cfg.screensaver_postprocess_policy,
        None,
        false,
        cfg.subtitle_placement,
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
    3
}
