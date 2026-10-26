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
        phase: i32, managed: bool, started: Instant,
        draw_count: u64, last_dimensions: Option<(i32, i32)>,
        time: i32, resolution: i32, mouse: i32, frame: i32,
        textures: Option<crate::manage_textures::TextureManager>,
    }
    thread_local! { static RESOURCES: RefCell<Option<Resources>> = const { RefCell::new(None) }; }

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
            let selected = std::env::var("SCREENSHAVER_LIBDECOR_TEST_SHADER")
                .ok().filter(|s| !s.trim().is_empty());
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
                // No policy ID is selected here: global/individual policy overrides
                // are deliberately NOT simulated in the diagnostic.
                let mut manager = crate::manage_textures::TextureManager::new(
                    crate::load_config::TexturePolicy {
                        global_texture: None,
                        global_palette: None,
                        texture_policy_entries: Vec::new(),
                    }
                );
                let channels = channel_usage.channels;
                if let Err(error) = manager.prepare_for_shader(name, channel_usage) {
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
                eprintln!("[LIBDECOR BRIDGE TEST] managed shader loaded: {name}; ISF inputs applied: {}; texture channels: {:?}; fallback policy", shader_inputs.len(), channels);
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
            RESOURCES.with(|r| *r.borrow_mut() = Some(Resources {
                program, vao, vbo, phase, managed, started: Instant::now(),
                time, resolution, mouse, frame,
                draw_count: 0, last_dimensions: None, textures,
            }));
            eprintln!("[LIBDECOR BRIDGE TEST] Rust GL renderer initialized via compile_shader::build_program");
            Ok(())
        });
        match result {
            Ok(Ok(())) => 0,
            Ok(Err(e)) => { eprintln!("[LIBDECOR BRIDGE TEST] Rust GL init failed: {e}"); -1 },
            Err(_) => { eprintln!("[LIBDECOR BRIDGE TEST] Rust GL init panicked"); -1 },
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn screenshaver_libdecor_rust_gl_draw(width: i32, height: i32, frame: u32) -> i32 {
        let result = std::panic::catch_unwind(|| RESOURCES.with(|slot| {
            let mut slot = slot.borrow_mut();
            let Some(r) = slot.as_mut() else { return -1 };
            if width <= 0 || height <= 0 { return -1; }
            if r.last_dimensions != Some((width, height)) {
                eprintln!("[LIBDECOR BRIDGE TEST] Rust viewport: {width} x {height}");
                r.last_dimensions = Some((width, height));
            }
            let elapsed = r.started.elapsed().as_secs_f32();
            if let Some(manager) = r.textures.as_mut() {
                if let Err(error) = manager.update_animations() {
                    eprintln!("[LIBDECOR BRIDGE TEST] Texture animation failed: {error}");
                    return -1;
                }
                manager.bind_channels();
            }
            unsafe {
                gl::Viewport(0, 0, width, height);
                gl::UseProgram(r.program);
                if r.managed {
                    gl::Uniform1f(r.time, elapsed);
                    gl::Uniform3f(r.resolution, width as f32, height as f32, 1.0);
                    gl::Uniform4f(r.mouse, 0.0, 0.0, 0.0, 0.0);
                    gl::Uniform1i(r.frame, frame.min(i32::MAX as u32) as i32);
                } else {
                    gl::Uniform1f(r.phase, elapsed);
                }
                gl::BindVertexArray(r.vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 3);
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    eprintln!("[LIBDECOR BRIDGE TEST] OpenGL draw error: 0x{error:04x}");
                    return -1;
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
            if let Some(mut r) = slot.borrow_mut().take() {
                eprintln!("[LIBDECOR BRIDGE TEST] Rust renderer release: frames={}, elapsed={:.2}s",
                    r.draw_count, r.started.elapsed().as_secs_f32());
                if let Some(manager) = r.textures.as_mut() { manager.delete_all(); }
                unsafe {
                    gl::UseProgram(0);
                    gl::DeleteBuffers(1, &r.vbo);
                    gl::DeleteVertexArrays(1, &r.vao);
                    gl::DeleteProgram(r.program);
                }
            }
        }));
    }
}
