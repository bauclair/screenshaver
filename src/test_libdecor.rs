//! Isolated, opt-in native libdecor test. Never used by production renderers.
#[cfg(feature = "libdecor-test")]
unsafe extern "C" {
    fn screenshaver_test_libdecor_run() -> libc::c_int;
}

pub(crate) fn run() -> Result<(), String> {
    if std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland") {
        return Err("This diagnostic requires a Wayland desktop session".into());
    }
    #[cfg(feature = "libdecor-test")]
    {
        println!("[LIBDECOR TEST] Starting isolated native test window");
        // SAFETY: the native entry point owns all Wayland resources it creates.
        let status = unsafe { screenshaver_test_libdecor_run() };
        if status == 0 { Ok(()) } else { Err(format!("Native test exited with status {status}")) }
    }
    #[cfg(not(feature = "libdecor-test"))]
    {
        Err("libdecor diagnostic was not compiled in. Build with: cargo build --features libdecor-test".into())
    }
}

// Stage 3C-2: isolated connection diagnostic. Does not create a surface or frame.
#[cfg(feature = "libdecor-bridge")]
unsafe extern "C" {
    fn screenshaver_decor_bridge_run_diagnostic() -> libc::c_int;
}

pub(crate) fn run_bridge() -> Result<(), String> {
    if std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland") {
        return Err("This diagnostic requires a Wayland desktop session".into());
    }
    #[cfg(feature = "libdecor-bridge")]
    {
        println!("[LIBDECOR BRIDGE TEST] Starting isolated connection diagnostic");
        // SAFETY: the native function creates, dispatches, and destroys its own
        // Wayland connection and libdecor context on the calling thread.
        let status = unsafe { screenshaver_decor_bridge_run_diagnostic() };
        if status == 0 {
            println!("[LIBDECOR BRIDGE TEST] PASS");
            Ok(())
        } else {
            Err(format!("Native bridge diagnostic exited with status {status}"))
        }
    }
    #[cfg(not(feature = "libdecor-bridge"))]
    {
        Err("libdecor bridge diagnostic was not compiled in. Build with: cargo build --release --features libdecor-bridge".into())
    }
}

// Stage 3C-6C: an opt-in Windowshader session passes its selected policy
// directly to the Rust diagnostic renderer on the same thread. The guard
// clears the selection even when native initialization returns an error.
#[cfg(feature = "libdecor-bridge")]
#[derive(Clone)]
struct WindowshaderNativeSelection {
    shader_name: String,
    policy_id: i64,
    source_path: std::path::PathBuf,
    texture_policy: crate::load_config::TexturePolicy,
    animation_speed: f32,
    postprocess_profile: crate::load_config::PostprocessProfile,
}

#[cfg(feature = "libdecor-bridge")]
thread_local! {
    static WINDOWSHADER_NATIVE_SELECTION: std::cell::RefCell<Option<WindowshaderNativeSelection>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "libdecor-bridge")]
pub(crate) struct WindowshaderNativeSelectionGuard;

#[cfg(feature = "libdecor-bridge")]
impl Drop for WindowshaderNativeSelectionGuard {
    fn drop(&mut self) {
        WINDOWSHADER_NATIVE_SELECTION.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(feature = "libdecor-bridge")]
pub(crate) fn install_windowshader_selection(
    shader_name: String,
    policy_id: i64,
    source_path: std::path::PathBuf,
    texture_policy: crate::load_config::TexturePolicy,
    animation_speed: f32,
    postprocess_profile: crate::load_config::PostprocessProfile,
) -> Result<WindowshaderNativeSelectionGuard, String> {
    WINDOWSHADER_NATIVE_SELECTION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err("Native windowshader selection already installed on this thread".into());
        }
        *slot = Some(WindowshaderNativeSelection {
            shader_name, policy_id, source_path, texture_policy, animation_speed, postprocess_profile,
        });
        Ok(WindowshaderNativeSelectionGuard)
    })
}

// Stage 28B (revised): lyrics-only state belongs to the native EGL owner thread.
// No policy-description, subtitle, or FPS-warning pills are installed here.
#[cfg(feature = "libdecor-bridge")]
thread_local! {
    static WINDOWSHADER_NATIVE_LYRICS: std::cell::RefCell<Option<crate::manage_lyrics::SharedLyricsState>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "libdecor-bridge")]
pub(crate) struct WindowshaderNativeLyricsGuard;

#[cfg(feature = "libdecor-bridge")]
impl Drop for WindowshaderNativeLyricsGuard {
    fn drop(&mut self) {
        WINDOWSHADER_NATIVE_LYRICS.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(feature = "libdecor-bridge")]
pub(crate) fn install_windowshader_lyrics(
    lyrics: Option<crate::manage_lyrics::SharedLyricsState>,
) -> WindowshaderNativeLyricsGuard {
    WINDOWSHADER_NATIVE_LYRICS.with(|slot| *slot.borrow_mut() = lyrics);
    WindowshaderNativeLyricsGuard
}

// Stage 28C2: panel geometry comes from the successfully built GL overlay,
// never a guessed height. All access stays on the native EGL/dispatch thread.
#[cfg(feature = "libdecor-bridge")]
#[derive(Clone)]
struct NativeLyricsInteraction {
    bounds: Option<(u32, u32, u32, u32)>, // width, surface height, panel height, navigation width
    seek: Option<crate::manage_lyrics::LyricsSeekHandle>,
    navigation: Option<crate::manage_lyrics::LyricsNavigationHandle>,
    state: Option<crate::manage_lyrics::SharedLyricsState>,
}
#[cfg(feature = "libdecor-bridge")]
thread_local! {
    static NATIVE_LYRICS_INTERACTION: std::cell::RefCell<NativeLyricsInteraction> =
        std::cell::RefCell::new(NativeLyricsInteraction {
            bounds: None, seek: None, navigation: None, state: None,
        });
}
#[cfg(feature = "libdecor-bridge")]
pub(crate) fn install_windowshader_lyrics_interaction(
    seek: Option<crate::manage_lyrics::LyricsSeekHandle>,
    navigation: Option<crate::manage_lyrics::LyricsNavigationHandle>,
    state: Option<crate::manage_lyrics::SharedLyricsState>,
) {
    NATIVE_LYRICS_INTERACTION.with(|slot| {
        *slot.borrow_mut() = NativeLyricsInteraction { bounds: None, seek, navigation, state };
    });
}
#[cfg(feature = "libdecor-bridge")]
pub(crate) fn clear_windowshader_lyrics_interaction() {
    NATIVE_LYRICS_INTERACTION.with(|slot| {
        let mut state = slot.borrow_mut();
        state.bounds = None;
        state.seek = None;
        state.navigation = None;
        state.state = None;
    });
}
#[cfg(feature = "libdecor-bridge")]
#[unsafe(no_mangle)]
pub extern "C" fn screenshaver_libdecor_rust_lyrics_pointer(
    x: f64, y: f64, pressed: std::ffi::c_int,
) -> std::ffi::c_int {
    if !x.is_finite() || !y.is_finite() { return 0; }
    NATIVE_LYRICS_INTERACTION.with(|slot| {
        let slot = slot.borrow();
        let Some((width, surface_height, panel_height, nav_width)) = slot.bounds else { return 0; };
        let bounds = crate::wayland_windowshader::WindowshaderLyricsBounds {
            width, surface_height, panel_height, navigation_width: nav_width,
        };
        if !bounds.contains(x, y) { return 0; }
        // Button 3 is a right-click. Match the XDG path: toggle playback
        // anywhere inside the lyric rectangle, including navigation buttons.
        if pressed == 3 {
            let accepted = slot.navigation.as_ref().is_some_and(|handle| handle.toggle_playback());
            eprintln!("[LIBDECOR LYRICS INPUT] toggle playback accepted={accepted}");
            return 1;
        }
        let snapshot = slot.state.as_ref().and_then(|s| s.lock().ok().map(|s| s.clone()));
        let Some(snapshot) = snapshot else { return 0; };
        if let Some(previous) = bounds.navigation_at(x, y) {
            let allowed = if previous { snapshot.can_go_previous } else { snapshot.can_go_next };
            if !allowed { return 0; }
            if pressed == 1 {
                let accepted = slot.navigation.as_ref().is_some_and(|handle| {
                    if previous { handle.previous() } else { handle.next() }
                });
                eprintln!("[LIBDECOR LYRICS INPUT] {} accepted={accepted}",
                    if previous { "previous" } else { "next" });
            }
            return 1;
        }
        let Some(fraction) = bounds.fraction_at(x, y) else { return 0; };
        if !snapshot.seekable || snapshot.progress.as_ref().is_none_or(|p| p.duration.is_zero()) {
            return 0;
        }
        if pressed == 1 {
            let accepted = slot.seek.as_ref().is_some_and(|handle| handle.request_fraction(fraction));
            eprintln!("[LIBDECOR LYRICS INPUT] seek fraction={fraction:.3} accepted={accepted}");
        }
        2
    })
}

// Stage 3C-6D2A: staging interface for the forthcoming rotation controller.
// This does not start a timer, alter supervisor behavior, or change the C ABI.
#[cfg(feature = "libdecor-bridge")]
#[allow(dead_code)]
pub(crate) fn replace_windowshader_policy(
    shader_name: String,
    policy_id: i64,
    source_path: std::path::PathBuf,
    texture_policy: crate::load_config::TexturePolicy,
    animation_speed: f32,
    postprocess_profile: crate::load_config::PostprocessProfile,
) -> Result<(), String> {
    rust_gl_diagnostic::replace_policy(WindowshaderNativeSelection {
        shader_name, policy_id, source_path, texture_policy, animation_speed, postprocess_profile,
    })
}

// Stage 3C-5F: Rust rendering callbacks run synchronously on the C diagnostic
// thread while its desktop OpenGL EGL context is current. Not production code.
#[cfg(feature = "libdecor-bridge")]
mod rust_gl_diagnostic {
    use std::cell::RefCell;
    use std::ffi::{c_char, c_void, CString};
    use std::time::Instant;

    unsafe extern "C" {
        fn screenshaver_libdecor_gl_proc_address(name: *const c_char) -> *const c_void;
    }

    struct Resources {
        program: u32, vao: u32, vbo: u32,
        phase: i32, managed: bool, started: Instant, animation_speed: f32,
        draw_count: u64, last_dimensions: Option<(i32, i32)>,
        time: i32, resolution: i32, mouse: i32, frame: i32,
        textures: Option<crate::manage_textures::TextureManager>,
        postprocess: Option<crate::postprocess_shader::PostprocessPipeline>,
        audio_demand: Option<crate::audio_backend::AudioDemandHandle>,
        audio_bands: Option<crate::audio_backend::SharedAudioBands>,
        audio_motion_effect: crate::render_audio_motion::AudioMotionEffect,
        audio_motion_state: crate::render_audio_motion::AudioMotionState,
        audio_last_update: Instant,
        lyrics_overlay: Option<crate::display_overlay::OpenGlOverlay>,
        lyrics_overlay_state: Option<(crate::manage_lyrics::LyricsState, u32, u32)>,
    }
    thread_local! {
        static RESOURCES: RefCell<Option<Resources>> = const { RefCell::new(None) };
        static REPLACEMENT_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
        static REPLACEMENT_POLICY: RefCell<Option<super::WindowshaderNativeSelection>> = const { RefCell::new(None) };
    }

    fn release_resources(mut r: Resources) {
        // Drop overlay while the native EGL context is current.
        r.postprocess.take();
        r.lyrics_overlay.take();
        if let Some(manager) = r.textures.as_mut() { manager.delete_all(); }
        unsafe {
            gl::UseProgram(0);
            gl::DeleteBuffers(1, &r.vbo);
            gl::DeleteVertexArrays(1, &r.vao);
            gl::DeleteProgram(r.program);
        }
    }

    // Called only on the native rendering thread, with the EGL context current.
    // A failed compilation or texture setup leaves the previous shader alive.
    // Stage 3C-6D2A: policy-aware replacement entry point. This is deliberately
    // not called by the native diagnostic timer. The rotation controller will
    // supply a fully resolved policy, rather than only a shader filename.
    // Must be called on the thread with the native EGL context current.
    pub(crate) fn replace_policy(selection: super::WindowshaderNativeSelection) -> Result<(), String> {
        if selection.policy_id <= 0 {
            return Err("Native replacement requires a database policy ID".into());
        }
        if selection.shader_name.is_empty() ||
            std::path::Path::new(&selection.shader_name).file_name()
                .and_then(|n| n.to_str()) != Some(selection.shader_name.as_str()) {
            return Err("Native replacement requires a managed shader filename".into());
        }
        if REPLACEMENT_POLICY.with(|slot| slot.borrow().is_some()) {
            return Err("Native policy replacement already in progress".into());
        }
        let previous = RESOURCES.with(|slot| slot.borrow_mut().take())
            .ok_or("No active shader to replace")?;
        let policy_id = selection.policy_id;
        let shader_name = selection.shader_name.clone();
        REPLACEMENT_POLICY.with(|slot| *slot.borrow_mut() = Some(selection));
        let initialized = screenshaver_libdecor_rust_gl_initialize();
        REPLACEMENT_POLICY.with(|slot| *slot.borrow_mut() = None);
        if initialized != 0 {
            if let Some(partial) = RESOURCES.with(|slot| slot.borrow_mut().take()) {
                release_resources(partial);
            }
            RESOURCES.with(|slot| *slot.borrow_mut() = Some(previous));
            return Err(format!("Policy {policy_id} shader {shader_name} failed preparation; previous policy retained"));
        }
        release_resources(previous);
        eprintln!("[LIBDECOR POLICY REPLACE] PASS policy={policy_id} shader={shader_name}");
        Ok(())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn screenshaver_libdecor_rust_gl_replace(name: *const c_char) -> i32 {
        let result = std::panic::catch_unwind(|| -> Result<(), String> {
            if name.is_null() { return Err("Null replacement shader name".into()); }
            let name = unsafe { std::ffi::CStr::from_ptr(name) }
                .to_str().map_err(|e| e.to_string())?.to_owned();
            if name.is_empty() || std::path::Path::new(&name).file_name()
                .and_then(|n| n.to_str()) != Some(name.as_str()) {
                return Err("Replacement must be a managed shader filename".into());
            }
            let previous = RESOURCES.with(|slot| slot.borrow_mut().take())
                .ok_or("No active shader to replace")?;
            REPLACEMENT_NAME.with(|slot| *slot.borrow_mut() = Some(name.clone()));
            let initialized = screenshaver_libdecor_rust_gl_initialize();
            REPLACEMENT_NAME.with(|slot| *slot.borrow_mut() = None);
            if initialized != 0 {
                // Initialize commits RESOURCES only after all preparation succeeds.
                // Defensive cleanup handles an unexpected partial commit.
                if let Some(partial) = RESOURCES.with(|slot| slot.borrow_mut().take()) {
                    release_resources(partial);
                }
                RESOURCES.with(|slot| *slot.borrow_mut() = Some(previous));
                return Err(format!("Could not prepare replacement {name}"));
            }
            release_resources(previous);
            eprintln!("[LIBDECOR REPLACE] PASS: {name}");
            Ok(())
        });
        match result {
            Ok(Ok(())) => 0,
            Ok(Err(error)) => { eprintln!("[LIBDECOR REPLACE] FAILED, previous shader retained: {error}"); -1 },
            Err(_) => { eprintln!("[LIBDECOR REPLACE] PANIC during replacement"); -1 },
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn screenshaver_libdecor_rust_gl_initialize() -> i32 {
        let result = std::panic::catch_unwind(|| -> Result<(), String> {
            if RESOURCES.with(|r| r.borrow().is_some()) {
                return Err("Diagnostic GL renderer already initialized".into());
            }
            gl::load_with(|name| {
                let c = CString::new(name).expect("GL symbol name");
                unsafe { screenshaver_libdecor_gl_proc_address(c.as_ptr()) }
            });
            // The optional value is a managed shader filename, not an arbitrary
            // filesystem path. The database supplies validated runtime GLSL.
            let replacement = REPLACEMENT_NAME.with(|slot| slot.borrow().clone());
            let pending_policy = REPLACEMENT_POLICY.with(|slot| slot.borrow().clone());
            let native_selection = if pending_policy.is_some() {
                pending_policy
            } else if replacement.is_some() {
                None
            } else {
                super::WINDOWSHADER_NATIVE_SELECTION.with(|slot| slot.borrow().clone())
            };
            let selected = native_selection.as_ref().map(|s| s.shader_name.clone()).or(replacement)
                .or_else(|| std::env::var("SCREENSHAVER_LIBDECOR_TEST_SHADER")
                    .ok().filter(|s| !s.trim().is_empty()));
            let (program, managed, mut textures) = if let Some(name) = selected.as_deref() {
                if std::path::Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name) {
                    return Err("Specify a managed shader filename, without a directory".into());
                }
                let (source, shader_inputs, channel_usage) = match crate::load_shader_source::load_managed_shader_source(name)? {
                    crate::load_shader_source::ShaderSourceResult::Ready {
                        source, channel_usage, shader_inputs, ..
                    } => {
                        (source, shader_inputs, channel_usage)
                    }
                    crate::load_shader_source::ShaderSourceResult::Rejected { reason, message } =>
                        return Err(format!("Shader rejected: {reason:?}: {message:?}")),
                    crate::load_shader_source::ShaderSourceResult::Unavailable { error } =>
                        return Err(error),
                };
                let program = crate::compile_shader::build_program(
                    crate::define_constants::VERTEX_SHADER, &source)?;
                // Match the production renderer: apply validated, database-stored
                // ISF input values after linking and while this EGL context is current.
                unsafe { gl::UseProgram(program); }
                crate::apply_shader_inputs::apply(program, &shader_inputs);
                let input_error = unsafe { gl::GetError() };
                if input_error != gl::NO_ERROR {
                    unsafe { gl::UseProgram(0); gl::DeleteProgram(program); }
                    return Err(format!("OpenGL error applying ISF inputs: 0x{input_error:04x}"));
                }
                // Use the production texture manager in this isolated EGL context.
                // Native windowshader sessions use the selected policy and its
                // texture configuration; standalone diagnostics keep fallback settings.
                let mut manager = crate::manage_textures::TextureManager::new(
                    native_selection.as_ref().map(|s| s.texture_policy.clone())
                        .unwrap_or_else(|| crate::load_config::TexturePolicy {
                            global_texture: None,
                            global_palette: None,
                            texture_policy_entries: Vec::new(),
                        })
                );
                let channels = channel_usage.channels;
                let prepared = if let Some(selection) = native_selection.as_ref() {
                    manager.prepare_for_policy_with_path(
                        selection.policy_id, name,
                        Some(selection.source_path.as_path()), channel_usage,
                    )
                } else {
                    manager.prepare_for_shader(name, channel_usage)
                };
                if let Err(error) = prepared {
                    manager.delete_all();
                    unsafe { gl::UseProgram(0); gl::DeleteProgram(program); }
                    return Err(format!("Texture preparation failed: {error}"));
                }
                manager.configure_program(program);
                let texture_error = unsafe { gl::GetError() };
                if texture_error != gl::NO_ERROR {
                    manager.delete_all();
                    unsafe { gl::UseProgram(0); gl::DeleteProgram(program); }
                    return Err(format!("Texture configuration GL error: 0x{texture_error:04x}"));
                }
                eprintln!("[LIBDECOR SHADER] loaded: {name}; ISF inputs applied: {}; texture channels: {:?}; policy={}",
                    shader_inputs.len(), channels,
                    native_selection.as_ref().map_or_else(|| "diagnostic fallback".to_string(), |s| s.policy_id.to_string()));
                (program, true, Some(manager))
            } else {
                let vertex = "#version 330 core\nlayout(location=0) in vec2 position; out vec2 uv; void main(){uv=position*0.5+0.5;gl_Position=vec4(position,0.0,1.0);}";
                let fragment = "#version 330 core\nin vec2 uv; uniform float phase; out vec4 frag_color; void main(){frag_color=vec4(uv.x,uv.y,0.4+0.3*sin(phase),1.0);}";
                (crate::compile_shader::build_program(vertex, fragment)?, false, None)
            };
            let mut vao = 0;
            let mut vbo = 0;
            let vertices: [f32; 6] = [-1., -1., 3., -1., -1., 3.];
            unsafe {
                gl::GenVertexArrays(1, &mut vao);
                gl::BindVertexArray(vao);
                gl::GenBuffers(1, &mut vbo);
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(gl::ARRAY_BUFFER, std::mem::size_of_val(&vertices) as isize,
                    vertices.as_ptr().cast(), gl::STATIC_DRAW);
                gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
                gl::EnableVertexAttribArray(0);
            }
            if vao == 0 || vbo == 0 || unsafe { gl::GetError() } != gl::NO_ERROR {
                unsafe {
                    if vbo != 0 { gl::DeleteBuffers(1, &vbo); }
                    if vao != 0 { gl::DeleteVertexArrays(1, &vao); }
                    if let Some(manager) = textures.as_mut() { manager.delete_all(); }
                    gl::DeleteProgram(program);
                }
                return Err("Rust GL resource initialization failed".into());
            }
            let phase = unsafe { gl::GetUniformLocation(program, c"phase".as_ptr()) };
            let time = unsafe { gl::GetUniformLocation(program, c"iTime".as_ptr()) };
            let resolution = unsafe { gl::GetUniformLocation(program, c"iResolution".as_ptr()) };
            let mouse = unsafe { gl::GetUniformLocation(program, c"iMouse".as_ptr()) };
            let frame = unsafe { gl::GetUniformLocation(program, c"iFrame".as_ptr()) };
            // Construct all effect resources with the native EGL context current.
            // Do not commit RESOURCES until the entire pipeline is ready.
            let mut postprocess = None;
            let mut audio_demand = None;
            let mut audio_bands = None;
            let mut audio_motion_effect = crate::render_audio_motion::AudioMotionEffect::Off;
            if let Some(selection) = native_selection.as_ref() {
                let profile = selection.postprocess_profile;
                audio_motion_effect = profile.audio_motion;
                // Initial native dimensions are established by libdecor later.
                // Resize on the first actual frame before rendering.
                match crate::postprocess_shader::PostprocessPipeline::new(960, 540, profile) {
                    Ok(pipeline) => postprocess = Some(pipeline),
                    Err(error) => {
                        unsafe {
                            gl::DeleteBuffers(1, &vbo);
                            gl::DeleteVertexArrays(1, &vao);
                            if let Some(manager) = textures.as_mut() { manager.delete_all(); }
                            gl::DeleteProgram(program);
                        }
                        return Err(format!("Native post-processing initialization failed: {error}"));
                    }
                }
                let demand = crate::audio_backend::register_audio_demand();
                demand.set_required(matches!(profile.bloom.name(), "audio" | "spectral" | "loudness")
                    || profile.audio_motion.is_enabled());
                audio_bands = Some(crate::audio_backend::shared_audio_bands());
                audio_demand = Some(demand);
            }
            RESOURCES.with(|r| *r.borrow_mut() = Some(Resources {
                program, vao, vbo, phase, managed, started: Instant::now(),
                animation_speed: native_selection.as_ref().map_or(1.0, |s| s.animation_speed),
                time, resolution, mouse, frame,
                draw_count: 0, last_dimensions: None, textures, postprocess, audio_demand, audio_bands,
                audio_motion_effect, audio_motion_state: Default::default(), audio_last_update: Instant::now(), lyrics_overlay: None, lyrics_overlay_state: None,
            }));
            eprintln!("[LIBDECOR RENDERER] initialized via compile_shader::build_program");
            Ok(())
        });
        match result {
            Ok(Ok(())) => 0,
            Ok(Err(e)) => { eprintln!("[LIBDECOR RENDERER] initialization failed: {e}"); -1 },
            Err(_) => { eprintln!("[LIBDECOR RENDERER] initialization panicked"); -1 },
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn screenshaver_libdecor_rust_gl_draw(width: i32, height: i32, frame: u32) -> i32 {
        let result = std::panic::catch_unwind(|| RESOURCES.with(|slot| {
            let mut slot = slot.borrow_mut();
            let Some(r) = slot.as_mut() else { return -1 };
            if width <= 0 || height <= 0 { return -1; }
            if r.last_dimensions != Some((width, height)) {
                eprintln!("[LIBDECOR RENDERER] viewport: {width} x {height}");
                r.last_dimensions = Some((width, height));
            }
            let elapsed = r.started.elapsed().as_secs_f32() * r.animation_speed;
            if let Some(manager) = r.textures.as_mut() {
                if let Err(error) = manager.update_animations() {
                    eprintln!("[LIBDECOR RENDERER] texture animation failed: {error}");
                    return -1;
                }
                manager.bind_channels();
            }
            let mut scene_dimensions = (width as u32, height as u32);
            if let Some(pipeline) = r.postprocess.as_mut() {
                if let Err(error) = pipeline.resize(width as u32, height as u32) {
                    eprintln!("[LIBDECOR POSTPROCESS] resize failed: {error}");
                    return -1;
                }
                if let Some(bands) = r.audio_bands.as_ref().and_then(|s| s.read().ok().map(|b| *b)) {
                    pipeline.set_audio_bands(bands);
                }
                let now = Instant::now();
                let frame_seconds = now.duration_since(r.audio_last_update).as_secs_f32();
                r.audio_last_update = now;
                match r.audio_motion_effect {
                    crate::render_audio_motion::AudioMotionEffect::WooferFromHell => {
                        let spectrum = crate::analyze_audio::shared_audio_motion_spectrum()
                            .read().ok().map(|v| *v).unwrap_or_default();
                        let vocal = crate::manage_lyrics::shared_audio_motion_vocal_timing_state()
                            .lock().ok().map(|v| v.clone()).unwrap_or_default();
                        pipeline.set_audio_motion_scale(r.audio_motion_state.update(spectrum, frame_seconds, &vocal));
                    }
                    crate::render_audio_motion::AudioMotionEffect::FftMirrorWarp => {
                        let spectrum = crate::analyze_audio::shared_audio_motion_fft_trace()
                            .read().ok().map(|v| *v).unwrap_or_default();
                        pipeline.set_audio_motion_fft_trace(r.audio_motion_state.update_fft_mirror_warp(spectrum, frame_seconds));
                    }
                    crate::render_audio_motion::AudioMotionEffect::PolarPropeller => {
                        let spectrum = crate::analyze_audio::shared_audio_motion_fft_trace()
                            .read().ok().map(|v| *v).unwrap_or_default();
                        pipeline.set_audio_motion_fft_trace(r.audio_motion_state.update_polar_propeller(spectrum, frame_seconds));
                        pipeline.set_audio_motion_rotation_turns(r.audio_motion_state.polar_rotation_turns());
                    }
                    crate::render_audio_motion::AudioMotionEffect::Off => r.audio_motion_state.reset(),
                }
                pipeline.bind_scene_target();
                scene_dimensions = pipeline.scene_dimensions();
            }
            unsafe {
                gl::Viewport(0, 0, scene_dimensions.0 as i32, scene_dimensions.1 as i32);
                gl::Disable(gl::BLEND);
                gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::FALSE);
                gl::UseProgram(r.program);
                if r.managed {
                    gl::Uniform1f(r.time, elapsed);
                    gl::Uniform3f(r.resolution, scene_dimensions.0 as f32, scene_dimensions.1 as f32, 1.0);
                    gl::Uniform4f(r.mouse, 0.0, 0.0, 0.0, 0.0);
                    gl::Uniform1i(r.frame, frame.min(i32::MAX as u32) as i32);
                } else {
                    gl::Uniform1f(r.phase, elapsed);
                }
                gl::BindVertexArray(r.vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 3);
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    eprintln!("[LIBDECOR RENDERER] OpenGL draw error: 0x{error:04x}");
                    return -1;
                }
            }
            if let Some(pipeline) = r.postprocess.as_ref() {
                pipeline.present_scene();
                let error = unsafe { gl::GetError() };
                if error != gl::NO_ERROR {
                    eprintln!("[LIBDECOR POSTPROCESS] presentation GL error: 0x{error:04x}");
                    return -1;
                }
            }
            // Stage 28B revised: synchronized lyrics only, matching the XDG
            // windowshader composition. MPRIS state is produced asynchronously;
            // this GL callback only snapshots it and never performs D-Bus work.
            let lyrics = super::WINDOWSHADER_NATIVE_LYRICS.with(|slot| {
                slot.borrow().as_ref().and_then(|shared| shared.lock().ok().map(|s| s.clone()))
            });
            if let Some(lyrics) = lyrics {
                if lyrics.current.is_none() && lyrics.progress.is_none() {
                    r.lyrics_overlay.take();
                    r.lyrics_overlay_state = None;
                    super::NATIVE_LYRICS_INTERACTION.with(|slot| slot.borrow_mut().bounds = None);
                } else {
                    let rebuild = r.lyrics_overlay_state.as_ref().is_none_or(|(old, ow, oh)| {
                        !old.same_text(&lyrics)
                            || old.can_go_previous != lyrics.can_go_previous
                            || old.can_go_next != lyrics.can_go_next
                            || *ow != width as u32 || *oh != height as u32
                    });
                    if rebuild {
                        // Destroy the previous texture/program before creating a new one.
                        r.lyrics_overlay.take();
                        r.lyrics_overlay_state = None;
                        let constructed = crate::construct_text_overlay::construct_lyrics_panel_bilingual_with_navigation(
                            lyrics.previous.as_deref(),
                            lyrics.current.as_deref(),
                            lyrics.next.as_deref(),
                            lyrics.secondary_current.as_deref(),
                            width as u32, height as u32,
                            lyrics.can_go_previous, lyrics.can_go_next,
                        );
                        match constructed.and_then(|panel| {
                            crate::display_overlay::OpenGlOverlay::new_from_constructed(
                                panel, crate::parse_subtitle_placement::parse(None).placement,
                            )
                        }) {
                            Ok(overlay) => {
                                super::NATIVE_LYRICS_INTERACTION.with(|slot| {
                                    slot.borrow_mut().bounds = Some((
                                        width as u32, height as u32,
                                        overlay.lyrics_panel_height(), overlay.lyrics_navigation_width(),
                                    ));
                                });
                                r.lyrics_overlay = Some(overlay);
                                r.lyrics_overlay_state = Some((lyrics.clone(), width as u32, height as u32));
                                eprintln!("[LIBDECOR LYRICS] overlay updated: {width} x {height}");
                            }
                            Err(error) => {
                                // Lyrics failure must not terminate shader presentation.
                                super::NATIVE_LYRICS_INTERACTION.with(|slot| slot.borrow_mut().bounds = None);
                                eprintln!("[LIBDECOR LYRICS] overlay unavailable: {error}");
                            }
                        }
                    }
                    if let Some(overlay) = r.lyrics_overlay.as_ref() {
                        overlay.display_bottom_edge(width as u32, height as u32);
                        if let Some(progress) = lyrics.progress.as_ref() {
                            overlay.display_progress_bottom_edge(width as u32, progress.fraction());
                        }
                        let error = unsafe { gl::GetError() };
                        if error != gl::NO_ERROR {
                            eprintln!("[LIBDECOR LYRICS] GL error: 0x{error:04x}");
                            return -1;
                        }
                    }
                }
            }
            r.draw_count = r.draw_count.saturating_add(1);
            0
        }));
        result.unwrap_or(-1)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn screenshaver_libdecor_rust_gl_release() {
        let _ = std::panic::catch_unwind(|| RESOURCES.with(|slot| {
                if let Some(r) = slot.borrow_mut().take() {
                    eprintln!("[LIBDECOR RENDERER] released: frames={}, elapsed={:.2}s",
                        r.draw_count, r.started.elapsed().as_secs_f32());
                    release_resources(r);
                }
        }));
    }
}
