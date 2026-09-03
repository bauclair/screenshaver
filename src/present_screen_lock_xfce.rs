use std::env;
use std::ffi::{c_void, CStr};
use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_long, c_uchar, c_ulong};
use std::path::Path;
use std::time::{Duration, Instant};

use x11::glx;
use x11::xlib;

const XSCREENSAVER_WINDOW_ENV: &str = "XSCREENSAVER_WINDOW";
const XFCE_DIAGNOSTIC_POLL_INTERVAL: Duration = Duration::from_millis(250);
const XFCE_DIAGNOSTIC_PROPERTY_LENGTH: c_long = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct X11PropertySnapshot {
    atom: xlib::Atom,
    name: String,
    actual_type: xlib::Atom,
    format: c_int,
    item_count: c_ulong,
    bytes_after: c_ulong,
    value_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XfceWindowSnapshot {
    parent_window: xlib::Window,
    root_window: xlib::Window,
    parent_x: c_int,
    parent_y: c_int,
    parent_width: c_int,
    parent_height: c_int,
    parent_depth: c_int,
    parent_map_state: c_int,
    parent_child_count: u32,
    input_focus: xlib::Window,
    input_focus_revert_to: c_int,
    properties: Vec<X11PropertySnapshot>,
}

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

    let mut diagnostic_state =
        match XfceLockDiagnosticState::new(
            logfile,
            display,
            window,
        ) {
            Ok(state) => Some(state),

            Err(error) => {
                crate::logger::warning(
                    logfile,
                    &format!(
                        "[LOCK] XFCE authentication-state reconnaissance unavailable; shader presentation will continue: {}",
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
            diagnostic_state.as_mut()
        {
            state.poll_if_due(
                logfile,
                display,
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


struct XfceLockDiagnosticState {
    presentation_window: xlib::Window,
    last_snapshot: XfceWindowSnapshot,
    next_poll: Instant,
}


impl XfceLockDiagnosticState {
    fn new(
        logfile: &Path,
        display: *mut xlib::Display,
        presentation_window: xlib::Window,
    ) -> Result<Self, String> {
        let snapshot =
            capture_xfce_window_snapshot(
                display,
                presentation_window,
            )?;

        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance baseline: presentation=0x{:X}, parent=0x{:X}, root=0x{:X}, parent_geometry={}x{}+{}+{}, depth={}, map_state={}, child_count={}, input_focus=0x{:X}, revert_to={}, property_count={}",
                presentation_window,
                snapshot.parent_window,
                snapshot.root_window,
                snapshot.parent_width,
                snapshot.parent_height,
                snapshot.parent_x,
                snapshot.parent_y,
                snapshot.parent_depth,
                snapshot.parent_map_state,
                snapshot.parent_child_count,
                snapshot.input_focus,
                snapshot.input_focus_revert_to,
                snapshot.properties.len(),
            ),
        );

        log_property_snapshot(
            logfile,
            "baseline",
            &snapshot.properties,
        );

        Ok(
            Self {
                presentation_window,
                last_snapshot: snapshot,
                next_poll: Instant::now()
                    + XFCE_DIAGNOSTIC_POLL_INTERVAL,
            }
        )
    }


    fn poll_if_due(
        &mut self,
        logfile: &Path,
        display: *mut xlib::Display,
    ) {
        let now =
            Instant::now();

        if now < self.next_poll {
            return;
        }

        self.next_poll =
            now + XFCE_DIAGNOSTIC_POLL_INTERVAL;

        let current =
            match capture_xfce_window_snapshot(
                display,
                self.presentation_window,
            ) {
                Ok(snapshot) => snapshot,

                Err(error) => {
                    crate::logger::warning(
                        logfile,
                        &format!(
                            "[LOCK] XFCE auth reconnaissance poll failed: {}",
                            error,
                        ),
                    );

                    return;
                }
            };

        if current == self.last_snapshot {
            return;
        }

        log_snapshot_changes(
            logfile,
            &self.last_snapshot,
            &current,
        );

        self.last_snapshot =
            current;
    }
}


fn capture_xfce_window_snapshot(
    display: *mut xlib::Display,
    presentation_window: xlib::Window,
) -> Result<XfceWindowSnapshot, String> {
    let (
        root_window,
        parent_window,
        _,
    ) =
        query_window_tree(
            display,
            presentation_window,
        )?;

    if parent_window == 0 {
        return Err(
            format!(
                "XFCE presentation window 0x{:X} has no parent window",
                presentation_window,
            )
        );
    }

    let mut parent_attributes =
        MaybeUninit::<xlib::XWindowAttributes>::uninit();

    let status =
        unsafe {
            xlib::XGetWindowAttributes(
                display,
                parent_window,
                parent_attributes.as_mut_ptr(),
            )
        };

    if status == 0 {
        return Err(
            format!(
                "XGetWindowAttributes failed for XFCE parent window 0x{:X}",
                parent_window,
            )
        );
    }

    let parent_attributes =
        unsafe {
            parent_attributes.assume_init()
        };

    let (
        _,
        _,
        parent_child_count,
    ) =
        query_window_tree(
            display,
            parent_window,
        )?;

    let mut input_focus =
        0 as xlib::Window;

    let mut input_focus_revert_to =
        0 as c_int;

    unsafe {
        xlib::XGetInputFocus(
            display,
            &mut input_focus,
            &mut input_focus_revert_to,
        );
    }

    let properties =
        capture_window_properties(
            display,
            parent_window,
        )?;

    Ok(
        XfceWindowSnapshot {
            parent_window,
            root_window,
            parent_x: parent_attributes.x,
            parent_y: parent_attributes.y,
            parent_width: parent_attributes.width,
            parent_height: parent_attributes.height,
            parent_depth: parent_attributes.depth,
            parent_map_state: parent_attributes.map_state,
            parent_child_count,
            input_focus,
            input_focus_revert_to,
            properties,
        }
    )
}


fn query_window_tree(
    display: *mut xlib::Display,
    window: xlib::Window,
) -> Result<(xlib::Window, xlib::Window, u32), String> {
    let mut root_return =
        0 as xlib::Window;

    let mut parent_return =
        0 as xlib::Window;

    let mut children_return: *mut xlib::Window =
        std::ptr::null_mut();

    let mut child_count =
        0u32;

    let status =
        unsafe {
            xlib::XQueryTree(
                display,
                window,
                &mut root_return,
                &mut parent_return,
                &mut children_return,
                &mut child_count,
            )
        };

    if !children_return.is_null() {
        unsafe {
            xlib::XFree(
                children_return as *mut c_void
            );
        }
    }

    if status == 0 {
        return Err(
            format!(
                "XQueryTree failed for X11 window 0x{:X}",
                window,
            )
        );
    }

    Ok(
        (
            root_return,
            parent_return,
            child_count,
        )
    )
}


fn capture_window_properties(
    display: *mut xlib::Display,
    window: xlib::Window,
) -> Result<Vec<X11PropertySnapshot>, String> {
    let mut property_count =
        0 as c_int;

    let property_atoms =
        unsafe {
            xlib::XListProperties(
                display,
                window,
                &mut property_count,
            )
        };

    if property_count < 0 {
        return Err(
            format!(
                "XListProperties returned an invalid property count for XFCE parent window 0x{:X}",
                window,
            )
        );
    }

    if property_atoms.is_null() {
        return Ok(
            Vec::new()
        );
    }

    let atoms =
        unsafe {
            std::slice::from_raw_parts(
                property_atoms,
                property_count as usize,
            )
        };

    let mut properties =
        Vec::with_capacity(
            atoms.len()
        );

    for atom in atoms {
        properties.push(
            capture_property_snapshot(
                display,
                window,
                *atom,
            )
        );
    }

    unsafe {
        xlib::XFree(
            property_atoms as *mut c_void
        );
    }

    properties.sort_by(
        |left, right| {
            left.atom.cmp(
                &right.atom
            )
        }
    );

    Ok(
        properties
    )
}


fn capture_property_snapshot(
    display: *mut xlib::Display,
    window: xlib::Window,
    property_atom: xlib::Atom,
) -> X11PropertySnapshot {
    let name =
        atom_name(
            display,
            property_atom,
        );

    let mut actual_type =
        0 as xlib::Atom;

    let mut actual_format =
        0 as c_int;

    let mut item_count =
        0 as c_ulong;

    let mut bytes_after =
        0 as c_ulong;

    let mut property_data: *mut c_uchar =
        std::ptr::null_mut();

    let status =
        unsafe {
            xlib::XGetWindowProperty(
                display,
                window,
                property_atom,
                0,
                XFCE_DIAGNOSTIC_PROPERTY_LENGTH,
                xlib::False,
                0,
                &mut actual_type,
                &mut actual_format,
                &mut item_count,
                &mut bytes_after,
                &mut property_data,
            )
        };

    let value_hash =
        if status == 0 && !property_data.is_null() {
            let byte_length =
                property_data_byte_length(
                    actual_format,
                    item_count,
                );

            let bytes =
                unsafe {
                    std::slice::from_raw_parts(
                        property_data as *const u8,
                        byte_length,
                    )
                };

            hash_bytes(
                bytes
            )
        } else {
            0
        };

    if !property_data.is_null() {
        unsafe {
            xlib::XFree(
                property_data as *mut c_void
            );
        }
    }

    X11PropertySnapshot {
        atom: property_atom,
        name,
        actual_type,
        format: actual_format,
        item_count,
        bytes_after,
        value_hash,
    }
}


fn property_data_byte_length(
    format: c_int,
    item_count: c_ulong,
) -> usize {
    let item_size =
        match format {
            8 => 1,
            16 => std::mem::size_of::<u16>(),
            32 => std::mem::size_of::<c_long>(),
            _ => 0,
        };

    (item_count as usize)
        .saturating_mul(
            item_size
        )
}


fn atom_name(
    display: *mut xlib::Display,
    atom: xlib::Atom,
) -> String {
    let raw_name =
        unsafe {
            xlib::XGetAtomName(
                display,
                atom,
            )
        };

    if raw_name.is_null() {
        return format!(
            "ATOM_{}",
            atom,
        );
    }

    let name =
        unsafe {
            CStr::from_ptr(
                raw_name
            )
        }
        .to_string_lossy()
        .into_owned();

    unsafe {
        xlib::XFree(
            raw_name as *mut c_void
        );
    }

    name
}


fn hash_bytes(
    bytes: &[u8],
) -> u64 {
    let mut hash =
        0xcbf29ce484222325u64;

    for byte in bytes {
        hash ^=
            *byte as u64;

        hash =
            hash.wrapping_mul(
                0x100000001b3
            );
    }

    hash
}


fn log_property_snapshot(
    logfile: &Path,
    label: &str,
    properties: &[X11PropertySnapshot],
) {
    for property in properties {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance {} property: name='{}', atom={}, type={}, format={}, items={}, bytes_after={}, hash=0x{:016X}",
                label,
                property.name,
                property.atom,
                property.actual_type,
                property.format,
                property.item_count,
                property.bytes_after,
                property.value_hash,
            ),
        );
    }
}


fn log_snapshot_changes(
    logfile: &Path,
    previous: &XfceWindowSnapshot,
    current: &XfceWindowSnapshot,
) {
    if previous.parent_window != current.parent_window {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: parent_window 0x{:X} -> 0x{:X}",
                previous.parent_window,
                current.parent_window,
            ),
        );
    }

    if previous.root_window != current.root_window {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: root_window 0x{:X} -> 0x{:X}",
                previous.root_window,
                current.root_window,
            ),
        );
    }

    if previous.parent_x != current.parent_x
        || previous.parent_y != current.parent_y
        || previous.parent_width != current.parent_width
        || previous.parent_height != current.parent_height
    {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: parent_geometry {}x{}+{}+{} -> {}x{}+{}+{}",
                previous.parent_width,
                previous.parent_height,
                previous.parent_x,
                previous.parent_y,
                current.parent_width,
                current.parent_height,
                current.parent_x,
                current.parent_y,
            ),
        );
    }

    if previous.parent_depth != current.parent_depth {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: parent_depth {} -> {}",
                previous.parent_depth,
                current.parent_depth,
            ),
        );
    }

    if previous.parent_map_state != current.parent_map_state {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: parent_map_state {} -> {}",
                previous.parent_map_state,
                current.parent_map_state,
            ),
        );
    }

    if previous.parent_child_count != current.parent_child_count {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: parent_child_count {} -> {}",
                previous.parent_child_count,
                current.parent_child_count,
            ),
        );
    }

    if previous.input_focus != current.input_focus
        || previous.input_focus_revert_to != current.input_focus_revert_to
    {
        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] XFCE auth reconnaissance change: input_focus 0x{:X}/{} -> 0x{:X}/{}",
                previous.input_focus,
                previous.input_focus_revert_to,
                current.input_focus,
                current.input_focus_revert_to,
            ),
        );
    }

    log_property_changes(
        logfile,
        &previous.properties,
        &current.properties,
    );
}


fn log_property_changes(
    logfile: &Path,
    previous: &[X11PropertySnapshot],
    current: &[X11PropertySnapshot],
) {
    for old_property in previous {
        match current
            .iter()
            .find(
                |new_property| {
                    new_property.atom == old_property.atom
                }
            )
        {
            Some(new_property) => {
                if new_property != old_property {
                    crate::logger::information(
                        logfile,
                        &format!(
                            "[LOCK] XFCE auth reconnaissance property changed: name='{}', atom={}, type {}->{}, format {}->{}, items {}->{}, bytes_after {}->{}, hash 0x{:016X}->0x{:016X}",
                            old_property.name,
                            old_property.atom,
                            old_property.actual_type,
                            new_property.actual_type,
                            old_property.format,
                            new_property.format,
                            old_property.item_count,
                            new_property.item_count,
                            old_property.bytes_after,
                            new_property.bytes_after,
                            old_property.value_hash,
                            new_property.value_hash,
                        ),
                    );
                }
            }

            None => {
                crate::logger::information(
                    logfile,
                    &format!(
                        "[LOCK] XFCE auth reconnaissance property removed: name='{}', atom={}",
                        old_property.name,
                        old_property.atom,
                    ),
                );
            }
        }
    }

    for new_property in current {
        if previous
            .iter()
            .all(
                |old_property| {
                    old_property.atom != new_property.atom
                }
            )
        {
            crate::logger::information(
                logfile,
                &format!(
                    "[LOCK] XFCE auth reconnaissance property added: name='{}', atom={}, type={}, format={}, items={}, bytes_after={}, hash=0x{:016X}",
                    new_property.name,
                    new_property.atom,
                    new_property.actual_type,
                    new_property.format,
                    new_property.item_count,
                    new_property.bytes_after,
                    new_property.value_hash,
                ),
            );
        }
    }
}
