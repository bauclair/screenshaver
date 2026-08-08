//! Full-screen interactive shader-edit rendering session.
//!
//! This checkpoint renders one optional shader continuously beneath an empty
//! movable and resizable egui editor window. When no shader is supplied, the
//! session displays a black full-screen background. Ordinary keyboard and mouse
//! activity do not terminate edit mode.

use std::collections::VecDeque;
use std::path::{
    Path,
    PathBuf,
};
use std::time::{
    Duration,
    Instant,
};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{
    FullscreenType,
    GLProfile,
};
use crate::parse_texture_specification::TextureSpecification;
use crate::editor_layout::EditWindowOverlay;


const FPS_AVERAGE_WINDOW: Duration =
    Duration::from_secs(5);

const FPS_CRITICAL_BLINK_INTERVAL: Duration =
    Duration::from_millis(500);


const FILE_DIALOG_FULLSCREEN_RESTORE_DELAY: Duration =
    Duration::from_millis(125);


const RECENT_SHADER_LIMIT: usize =
    8;


struct FrameTimeWindow {
    samples: VecDeque<(Instant, Duration)>,
    total: Duration,
}


impl FrameTimeWindow {
    fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            total: Duration::ZERO,
        }
    }


    fn record(
        &mut self,
        elapsed: Duration,
        configured_fps: u32,
    ) -> crate::fps_monitor::FpsWarningState {
        let now = Instant::now();

        self.samples.push_back(
            (now, elapsed)
        );

        self.total += elapsed;

        while let Some((timestamp, duration)) =
            self.samples.front().copied()
        {
            if now.duration_since(timestamp)
                <= FPS_AVERAGE_WINDOW
            {
                break;
            }

            self.samples.pop_front();
            self.total = self.total.saturating_sub(
                duration
            );
        }

        let sample_count =
            self.samples.len() as u32;

        if sample_count == 0 {
            return crate::fps_monitor::FpsWarningState::Normal;
        }

        let average_seconds =
            self.total.as_secs_f64()
                / sample_count as f64;

        let ideal_seconds =
            1.0 / configured_fps.max(1) as f64;

        if average_seconds
            > ideal_seconds * 2.0
        {
            crate::fps_monitor::FpsWarningState::Critical
        } else if average_seconds
            > ideal_seconds * 1.5
        {
            crate::fps_monitor::FpsWarningState::Warning
        } else {
            crate::fps_monitor::FpsWarningState::Normal
        }
    }
}


struct ActivePreviewShader {
    path:
        PathBuf,

    shader_name:
        String,

    program:
        u32,

    channel_usage:
        crate::preprocess_shader::ShaderChannelUsage,

    shader_inputs:
        Vec<crate::isf_types::ShaderInput>,

    texture_manager:
        crate::manage_textures::TextureManager,

    overlay_descriptor:
        crate::construct_text_overlay::OverlayDescriptor,

    subtitle_overlay:
        Option<
            crate::display_overlay::OpenGlOverlay
        >,

    overlay_output_size:
        (
            u32,
            u32,
        ),

    fps_warning_state:
        crate::fps_monitor::FpsWarningState,

    fps_blink_visible:
        bool,

    last_fps_blink:
        Instant,

    frame_times:
        FrameTimeWindow,

    start_time:
        Instant,

    previous_frame:
        Instant,

    frame:
        i32,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorTargetRestriction {
    Unrestricted,
    WallpaperOnly,
    ScreensaverOnly,
}

pub fn run(
    shader_argument: Option<String>,
) -> Result<(), String> {

    let Some(shader_argument) =
        shader_argument
    else {
        return run_empty_session();
    };


    match crate::preview_shader_directory::resolve_preview_target(
        &shader_argument
    )? {

        crate::preview_shader_directory::PreviewTarget::File(
            path
        ) => {

            run_paths(
                vec![
                    path
                ],
                None,
                None,
                None,
                None,
                None,
                EditorTargetRestriction::Unrestricted,
                None,
            )
        }

        crate::preview_shader_directory::PreviewTarget::Directory(
            path
        ) => {

            Err(
                format!(
                    "--edit-shader requires a shader file, not a directory: {}",
                    path.display(),
                )
            )
        }
    }
}


pub fn run_wallpaper_only(
    shader_path: PathBuf,
) -> Result<(), String> {
    run_paths(
        vec![shader_path],
        None,
        None,
        None,
        None,
        None,
        EditorTargetRestriction::WallpaperOnly,
        Some(
            crate::editor_layout::PolicyTarget::Wallpaper
        ),
    )
}


pub fn run_screensaver_only(
    shader_path: PathBuf,
) -> Result<(), String> {
    run_paths(
        vec![shader_path],
        None,
        None,
        None,
        None,
        None,
        EditorTargetRestriction::ScreensaverOnly,
        Some(
            crate::editor_layout::PolicyTarget::Screensaver
        ),
    )
}

fn run_empty_session() -> Result<(), String> {

    let wallpaper_pause_guard =
        crate::control_wallpaper::WallpaperPauseGuard::acquire();

    let config_result =
        crate::load_config::load_config(
            &crate::locate_paths::config_path()
        )?;

    let mut config =
        config_result.config;

    let policy_display_rows =
        build_policy_display_rows(
            &config
        );

    let mut recent_shader_paths =
        load_recent_shader_paths();


    let sdl =
        sdl2::init()
            .map_err(
                |error| {
                    format!(
                        "SDL initialization failed: {}",
                        error,
                    )
                }
            )?;


    let video =
        sdl.video()
            .map_err(
                |error| {
                    format!(
                        "SDL video initialization failed: {}",
                        error,
                    )
                }
            )?;


    {
        let gl_attr =
            video.gl_attr();

        gl_attr.set_context_profile(
            GLProfile::Core
        );

        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }


    let mut window =
        video
            .window(
                "Screenshaver Control Center",
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
                        "Unable to create edit-shader window: {}",
                        error,
                    )
                }
            )?;


    let gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create edit-shader OpenGL context: {}",
                        error,
                    )
                }
            )?;


    window.raise();

    let _ =
        window.set_fullscreen(
            FullscreenType::Desktop
        );


    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );


    let _ =
        video.gl_set_swap_interval(
            0
        );


    let mut edit_window =
        EditWindowOverlay::new(
            &video
        )?;


    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create edit-shader SDL event pump: {}",
                        error,
                    )
                }
            )?;


    let mut policy_open_request:
        Option<(
            PathBuf,
            Option<
                crate::editor_layout::PolicyTarget
            >,
        )> =
        None;


    'edit_session: loop {

        for event in
            event_pump.poll_iter()
        {
            edit_window.handle_event(
                &event
            );

            if edit_session_should_close(
                &event
            ) {
                break 'edit_session;
            }
        }


        let (
            width,
            height,
        ) =
            window.drawable_size();


        unsafe {
            gl::Viewport(
                0,
                0,
                width.min(i32::MAX as u32) as i32,
                height.min(i32::MAX as u32) as i32,
            );

            gl::ClearColor(
                0.0,
                0.0,
                0.0,
                1.0,
            );

            gl::Clear(
                gl::COLOR_BUFFER_BIT
            );
        }


        let editor_output =
            edit_window.display(
                &window,
                crate::define_constants::DEFAULT_RENDER_FPS,
                crate::define_constants::SCREENSAVER_SPEED_DEFAULT,
                crate::define_constants::RENDER_SCALE_DEFAULT,
                crate::editor_layout::AntiAliasingSelection::Fxaa,
                crate::editor_layout::DitheringSelection::Subtle,
                crate::editor_layout::ColorPrecisionSelection::Automatic,
                None,
                false,
                false,
                false,
                false,
                false,
                false,
                &recent_shader_paths,
                None,
                &policy_display_rows,
                Some(&config),
            );

        if editor_output.control_configuration_save_requested {
            if let Some(control_configuration) =
                editor_output.control_configuration.as_ref()
            {
                match save_control_configuration(
                    control_configuration,
                ) {
                    Ok(reloaded_config) => {
                        config =
                            reloaded_config;

                        edit_window.accept_control_configuration();

                        edit_window.set_status_message(
                            "Configuration saved."
                        );

                        log_information(
                            "[EDIT_SHADER] Configuration saved from empty Control Center session"
                        );
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            "Configuration save failed."
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to save configuration: {}",
                                error,
                            )
                        );
                    }
                }
            }
        }


        let recent_selected_path =
            editor_output.recent_shader_requested
                .and_then(
                    |index| {
                        recent_shader_paths
                            .get(index)
                            .cloned()
                    }
                );


        if editor_output.browse_shader_requested
            || recent_selected_path.is_some()
        {
            let selected_path =
                if editor_output.browse_shader_requested {
                    let starting_directory =
                        crate::locate_paths::screensaver_shader_dir();

                    let selected_path =
                        rfd::FileDialog::new()
                            .add_filter(
                                "GL shader files",
                                &[
                                    "glsl",
                                    "fs",
                                ],
                            )
                            .set_directory(
                                &starting_directory
                            )
                            .pick_file();

                    if let Err(error) =
                        restore_editor_fullscreen(
                            &mut window
                        )
                    {
                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Immediate fullscreen restoration failed: {}",
                                error,
                            )
                        );
                    }

                    selected_path
                } else {
                    recent_selected_path
                };


            let Some(selected_path) =
                selected_path
            else {
                edit_window.set_status_message(
                    "Shader loading canceled."
                );

                continue;
            };


            if !selected_path.is_file() {
                recent_shader_paths.retain(
                    |path| {
                        path != &selected_path
                    }
                );

                let _ =
                    save_recent_shader_paths(
                        &recent_shader_paths
                    );

                edit_window.set_status_message(
                    format!(
                        "Shader file no longer exists: {}",
                        selected_path.display(),
                    )
                );

                continue;
            }


            promote_recent_shader_path(
                &mut recent_shader_paths,
                selected_path.clone(),
            );

            let _ =
                save_recent_shader_paths(
                    &recent_shader_paths
                );


            policy_open_request =
                Some(
                    (
                        selected_path,
                        None,
                    )
                );

            break 'edit_session;
        }


        if let Some((
            row,
            command,
        )) =
            editor_output
                .policy_row_command_requested
                .as_ref()
                .filter(
                    |(
                        _row,
                        command,
                    )| {
                        matches!(
                            *command,
                            crate::editor_layout::PolicyRowCommand::Edit
                                | crate::editor_layout::PolicyRowCommand::RefreshShader
                        )
                    }
                )
        {
            let selected_path =
                resolve_policy_shader_path(
                    &config,
                    row.policy_target,
                    &row.filename,
                );


            if !selected_path.is_file() {
                edit_window.set_status_message(
                    format!(
                        "Policy shader file is unavailable: {}",
                        selected_path.display(),
                    )
                );

                continue;
            }


            policy_open_request =
                Some(
                    (
                        selected_path,
                        Some(
                            row.policy_target
                        ),
                    )
                );

            break 'edit_session;
        }


        if let Some((
            target,
            index,
        )) =
            editor_output.control_single_recent_requested
        {
            if let Some(path) =
                recent_shader_paths
                    .get(index)
            {
                if let Some(filename) =
                    path.file_name()
                        .and_then(
                            |name| {
                                name.to_str()
                            }
                        )
                {
                    edit_window.set_control_single_filename(
                        target,
                        filename,
                    );

                    edit_window.set_status_message(
                        "Single shader selected."
                    );
                }
            }
        }


        if let Some(target) =
            editor_output.control_single_browse_requested
        {
            let starting_directory =
                match target {
                    crate::editor_layout::PolicyTarget::Screensaver => {
                        crate::locate_paths::screensaver_shader_dir()
                    }

                    crate::editor_layout::PolicyTarget::Wallpaper => {
                        crate::locate_paths::wallpaper_shader_dir()
                    }
                };

            let selected_path =
                rfd::FileDialog::new()
                    .add_filter(
                        "GL shader files",
                        &[
                            "glsl",
                            "fs",
                        ],
                    )
                    .set_directory(
                        &starting_directory
                    )
                    .pick_file();

            if let Some(selected_path) =
                selected_path
            {
                if let Some(filename) =
                    selected_path
                        .file_name()
                        .and_then(
                            |name| {
                                name.to_str()
                            }
                        )
                {
                    edit_window.set_control_single_filename(
                        target,
                        filename,
                    );

                    promote_recent_shader_path(
                        &mut recent_shader_paths,
                        selected_path.clone(),
                    );

                    let _ =
                        save_recent_shader_paths(
                            &recent_shader_paths
                        );

                    edit_window.set_status_message(
                        "Single shader selected."
                    );
                }
            } else {
                edit_window.set_status_message(
                    "Shader selection canceled."
                );
            }

            if let Err(error) =
                restore_editor_fullscreen(
                    &mut window
                )
            {
                log_warning(
                    &format!(
                        "[EDIT_SHADER] Immediate fullscreen restoration failed: {}",
                        error,
                    )
                );
            }
        }


        if !editor_output.window_open {
            break 'edit_session;
        }


        window.gl_swap_window();

        std::thread::sleep(
            Duration::from_millis(10)
        );
    }


    edit_window.destroy();


    // The empty Control Center session owns SDL's single EventPump.
    // Drop it, along with the empty-session window/context objects,
    // before starting a shader-loaded Control Center session.
    drop(
        event_pump
    );


    drop(
        gl_context
    );


    drop(
        window
    );


    drop(
        video
    );


    drop(
        sdl
    );


    drop(
        wallpaper_pause_guard
    );


    if let Some((
        shader_path,
        policy_target,
    )) =
        policy_open_request
    {
        return run_paths(
            vec![
                shader_path
            ],
            None,
            None,
            None,
            None,
            None,
            EditorTargetRestriction::Unrestricted,
            policy_target,
        );
    }


    Ok(())
}


fn restore_editor_fullscreen(
    window: &mut sdl2::video::Window,
) -> Result<(), String> {

    window
        .set_fullscreen(
            FullscreenType::Desktop
        )
        .map_err(
            |error| {
                format!(
                    "Unable to restore Screenshaver Control Center fullscreen state: {}",
                    error,
                )
            }
        )?;


    window.raise();


    Ok(())
}


fn edit_session_should_close(
    event: &Event,
) -> bool {

    matches!(
        event,
        Event::Quit {
            ..
        }
        | Event::KeyDown {
            keycode:
                Some(
                    Keycode::Escape
                    | Keycode::Q
                ),
            repeat: false,
            ..
        }
    )
}

fn run_paths(
    shader_paths: Vec<PathBuf>,
    shader_texture: Option<TextureSpecification>,
    shader_palette: Option<String>,
    interval_seconds: Option<u64>,
    command_line_fps: Option<u32>,
    animation_speed: Option<f32>,
    target_restriction: EditorTargetRestriction,
    requested_initial_target:
        Option<
            crate::editor_layout::PolicyTarget
        >,
) -> Result<(), String> {

    if shader_paths.is_empty() {
        return Err(
            "No shader path was supplied for editing"
                .to_string()
        );
    }


    let command_line_animation_speed =
        animation_speed;


    let config_result =
        crate::load_config::load_config(
            &crate::locate_paths::config_path()
        )?;


    let mut config =
        config_result.config;


    let mut policy_display_rows =
        build_policy_display_rows(
            &config
        );


    let mut recent_shader_paths =
        load_recent_shader_paths();


    let subtitles =
        config.subtitles;


    let subtitle_placement =
        config.subtitle_placement;


    let shader_name_hint =
        shader_paths
            .first()
            .and_then(
                |path| {
                    path.file_name()
                }
            )
            .and_then(
                |name| {
                    name.to_str()
                }
            )
            .unwrap_or("")
            .to_string();


    let mut screensaver_target_available =
        target_restriction
            == EditorTargetRestriction::Unrestricted
        || target_shader_path(
            crate::editor_layout::PolicyTarget::Screensaver,
            &shader_name_hint,
        )
        .is_file();


    let mut wallpaper_target_available =
        target_restriction
            == EditorTargetRestriction::Unrestricted
        || target_shader_path(
            crate::editor_layout::PolicyTarget::Wallpaper,
            &shader_name_hint,
        )
        .is_file();


    match target_restriction {
        EditorTargetRestriction::WallpaperOnly => {
            screensaver_target_available = false;
        }
        EditorTargetRestriction::ScreensaverOnly => {
            wallpaper_target_available = false;
        }
        EditorTargetRestriction::Unrestricted => {}
    }


    let initial_shader_path =
        shader_paths
            .first()
            .expect(
                "shader_paths was checked for emptiness"
            );


    let mut screensaver_policy_exists =
        screensaver_target_available
            && config.screensaver_policies
            .iter()
            .any(
                |policy| {
                    policy_applies_to_path(
                        policy,
                        crate::editor_layout::PolicyTarget::Screensaver,
                        initial_shader_path,
                    )
                }
            );


    let mut wallpaper_policy_exists =
        wallpaper_target_available
            && config.wallpaper_policies
            .iter()
            .any(
                |policy| {
                    policy_applies_to_path(
                        policy,
                        crate::editor_layout::PolicyTarget::Wallpaper,
                        initial_shader_path,
                    )
                }
            );


    let initial_editor_target =
        if let Some(
            requested_initial_target
        ) =
            requested_initial_target
        {
            Some(
                requested_initial_target
            )
        } else if target_restriction
            == EditorTargetRestriction::WallpaperOnly
        {
            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            )
        } else if target_restriction
            == EditorTargetRestriction::ScreensaverOnly
        {
            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            )
        } else if wallpaper_policy_exists {
            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            )
        } else if screensaver_policy_exists {
            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            )
        } else {
            None
        };


    let (
        mut global_rendered_fps,
        mut fps_policy_entries,
        mut texture_policy,
        mut postprocess_policy,
        mut animation_speed,
        startup_status,
    ) =
        match initial_editor_target {
            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ) => {
                (
                    config.wallpaper_fps_policy
                        .global_rendered_fps,
                    config.wallpaper_fps_policy
                        .fps_policy_entries
                        .clone(),
                    config.wallpaper_texture_policy
                        .clone(),
                    config.wallpaper_postprocess_policy
                        .clone(),
                    config.wallpaper_speed_policy
                        .animation_speed_for_shader(
                            &shader_name_hint,
                            command_line_animation_speed,
                        ),
                    "Loaded existing Wallpaper policy for this shader."
                        .to_string(),
                )
            }

            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ) => {
                (
                    config.global_rendered_fps,
                    config.screensaver_fps_policy_entries
                        .clone(),
                    config.texture_policy
                        .clone(),
                    config.screensaver_postprocess_policy
                        .clone(),
                    config.screensaver_speed_policy
                        .animation_speed_for_shader(
                            &shader_name_hint,
                            command_line_animation_speed,
                        ),
                    "Loaded existing Screensaver policy for this shader."
                        .to_string(),
                )
            }

            None => {
                (
                    config.global_rendered_fps,
                    config.screensaver_fps_policy_entries
                        .clone(),
                    config.texture_policy
                        .clone(),
                    config.screensaver_postprocess_policy
                        .clone(),
                    config.screensaver_speed_policy
                        .animation_speed_for_shader(
                            &shader_name_hint,
                            command_line_animation_speed,
                        ),
                    "No existing shader policy found. Select a policy target to create one."
                        .to_string(),
                )
            }
        };


    let preview_selection =
        parse_preview_selection(
            shader_texture.as_ref(),
            shader_palette.as_deref(),
        )?;


    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        &format!(
            "[EDIT_SHADER] Edit session contains {} shader path(s)",
            shader_paths.len(),
        ),
    );


    crate::logger::information(
        &logfile,
        &format!(
            "[EDIT_SHADER] Animation speed: {:.3}x",
            animation_speed,
        ),
    );


    let _wallpaper_pause_guard =
        if target_restriction
            == EditorTargetRestriction::Unrestricted
        {
            Some(
                crate::control_wallpaper::WallpaperPauseGuard::acquire()
            )
        } else {
            None
        };


    let sdl =
        sdl2::init()
            .map_err(
                |error| {
                    format!(
                        "SDL initialization failed: {}",
                        error,
                    )
                }
            )?;


    let video =
        sdl.video()
            .map_err(
                |error| {
                    format!(
                        "SDL video initialization failed: {}",
                        error,
                    )
                }
            )?;


    {
        let gl_attr =
            video.gl_attr();


        gl_attr.set_context_profile(
            GLProfile::Core
        );


        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }


    let mut window =
        video
            .window(
                "Screenshaver Control Center",
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
                        "Unable to create edit-shader window: {}",
                        error,
                    )
                }
            )?;


    let gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create OpenGL context: {}",
                        error,
                    )
                }
            )?;


    window.raise();


    let _ =
        window.set_fullscreen(
            FullscreenType::Desktop
        );


    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );


    let _ =
        video.gl_set_swap_interval(
            0
        );


    let mut edit_window =
        EditWindowOverlay::new(
            &video
        )?;


    let (
        width,
        height,
    ) =
        window.size();


    let (
        mut active,
        mut active_index,
    ) =
        load_first_usable_shader(
            &shader_paths,
            0,
            &texture_policy,
            preview_selection,
            subtitles,
            subtitle_placement,
            global_rendered_fps,
            &fps_policy_entries,
            command_line_fps,
            animation_speed,
            width,
            height,
        )?;


    let mut information_path =
        resolve_information_path(
            &active.path,
            &active.shader_name,
            initial_editor_target,
        );


    let initial_postprocess_profile =
        postprocess_policy.profile_for_shader(
            &active.shader_name
        );


    let mut live_postprocess_profile =
        initial_postprocess_profile;


    let mut render_scale =
        live_postprocess_profile.render_scale;


    let mut postprocess =
        crate::postprocess_shader::PostprocessPipeline::new(
            width,
            height,
            live_postprocess_profile,
        )?;


    let mut configured_fps =
        resolve_preview_fps(
            global_rendered_fps,
            &fps_policy_entries,
            command_line_fps,
            &active.shader_name,
        );


    let mut target_frame_time =
        Duration::from_secs_f64(
            1.0
                / configured_fps.max(1) as f64
        );


    edit_window.initialize_configuration(
        configured_fps,
        animation_speed,
        render_scale,
        initial_editor_target,
        anti_aliasing_selection_from_method(
            live_postprocess_profile.anti_aliasing
        ),
        dithering_selection_from_level(
            live_postprocess_profile.dithering
        ),
        color_precision_selection_from_policy(
            live_postprocess_profile.color_precision
        ),
        active.texture_manager
            .active_specification_selection(),
        startup_status,
    );


    let mut vao =
        0_u32;


    unsafe {
        gl::GenVertexArrays(
            1,
            &mut vao,
        );


        gl::BindVertexArray(
            vao
        );
    }


    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create SDL event pump: {}",
                        error,
                    )
                }
            )?;


    discard_startup_input(
        &mut event_pump
    );


    let mut last_switch =
        Instant::now();


    let mut fullscreen_restore_requested_at:
        Option<Instant> =
        None;


    let result =
        'preview: loop {

            for event in
                event_pump.poll_iter()
            {
                edit_window.handle_event(
                    &event
                );

                match event {

                    ref event
                        if edit_session_should_close(
                            event
                        ) =>
                    {
                        break 'preview Ok(());
                    }

                    _ => {
                        // Keyboard and mouse input remain active in edit mode
                        // and do not automatically terminate the session.
                    }
                }
            }


            if fullscreen_restore_requested_at
                .is_some_and(
                    |requested_at| {
                        requested_at.elapsed()
                            >= FILE_DIALOG_FULLSCREEN_RESTORE_DELAY
                    }
                )
            {
                if let Err(error) =
                    restore_editor_fullscreen(
                        &mut window
                    )
                {
                    log_warning(
                        &format!(
                            "[EDIT_SHADER] Deferred fullscreen restoration failed: {}",
                            error,
                        )
                    );
                }

                fullscreen_restore_requested_at =
                    None;
            }


            if let Some(interval) =
                interval_seconds
            {
                if last_switch.elapsed()
                    >= Duration::from_secs(
                        interval
                    )
                {
                    let next_start =
                        (
                            active_index
                                + 1
                        )
                            % shader_paths.len();


                    match load_first_usable_shader(
                        &shader_paths,
                        next_start,
                        &texture_policy,
                        preview_selection,
                        subtitles,
                        subtitle_placement,
                        global_rendered_fps,
                        &fps_policy_entries,
                        command_line_fps,
                        animation_speed,
                        window.size().0,
                        window.size().1,
                    ) {

                        Ok(
                            (
                                replacement,
                                replacement_index,
                            )
                        ) => {

                            destroy_active_shader(
                                &mut active
                            );


                            active =
                                replacement;


                            postprocess.set_profile(
                                postprocess_policy.profile_for_shader(
                                    &active.shader_name
                                )
                            )?;


                            configured_fps =
                                resolve_preview_fps(
                                    global_rendered_fps,
                                    &fps_policy_entries,
                                    command_line_fps,
                                    &active.shader_name,
                                );


                            target_frame_time =
                                Duration::from_secs_f64(
                                    1.0
                                        / configured_fps.max(1) as f64
                                );


                            active_index =
                                replacement_index;


                            last_switch =
                                Instant::now();


                            log_information(
                                &format!(
                                    "[EDIT_SHADER] Switched to {}",
                                    active.path.display(),
                                )
                            );
                        }

                        Err(error) => {

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Unable to select another usable shader: {}",
                                    error,
                                )
                            );


                            last_switch =
                                Instant::now();
                        }
                    }
                }
            }


            let frame_start =
                Instant::now();


            let (
                width,
                height,
            ) =
                window.size();


            let elapsed =
                active.start_time
                    .elapsed()
                    .as_secs_f32()
                    * animation_speed;


            let delta =
                active.previous_frame
                    .elapsed()
                    .as_secs_f32()
                    * animation_speed;


            let shader_render_start =
                Instant::now();


            postprocess.resize(
                width,
                height,
            )?;


            postprocess.bind_scene_target();


            let (
                scene_width,
                scene_height,
            ) =
                postprocess.scene_dimensions();


            unsafe {
                gl::ClearColor(
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                );


                gl::Clear(
                    gl::COLOR_BUFFER_BIT
                );


                gl::UseProgram(
                    active.program
                );


                active.texture_manager
                    .bind_channels();


                crate::apply_shader_inputs::apply(
                    active.program,
                    &active.shader_inputs,
                );


                gl::BindVertexArray(
                    vao
                );


                set_uniform_1f(
                    active.program,
                    b"iTime\0",
                    elapsed,
                );


                set_uniform_1f(
                    active.program,
                    b"iTimeDelta\0",
                    delta,
                );


                set_uniform_1i(
                    active.program,
                    b"iFrame\0",
                    active.frame,
                );


                set_uniform_3f(
                    active.program,
                    b"iResolution\0",
                    scene_width as f32,
                    scene_height as f32,
                    1.0,
                );


                set_uniform_4f(
                    active.program,
                    b"iMouse\0",
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                );


                gl::DrawArrays(
                    gl::TRIANGLES,
                    0,
                    3,
                );
            }


            postprocess.present_scene();


            unsafe {
                gl::Finish();
            }


            let warning_state =
                active.frame_times.record(
                    shader_render_start.elapsed(),
                    configured_fps,
                );


            let warning_changed =
                warning_state
                    != active.fps_warning_state;


            if warning_changed {
                active.fps_warning_state =
                    warning_state;

                active.fps_blink_visible =
                    true;

                active.last_fps_blink =
                    Instant::now();
            }


            let mut blink_changed =
                false;


            if active.fps_warning_state
                == crate::fps_monitor::FpsWarningState::Critical
                && active.last_fps_blink.elapsed()
                    >= FPS_CRITICAL_BLINK_INTERVAL
            {
                active.fps_blink_visible =
                    !active.fps_blink_visible;

                active.last_fps_blink =
                    Instant::now();

                blink_changed =
                    true;
            }


            let overlay_warning_state =
                if active.fps_warning_state
                    == crate::fps_monitor::FpsWarningState::Critical
                    && !active.fps_blink_visible
                {
                    crate::fps_monitor::FpsWarningState::CriticalHidden
                } else {
                    active.fps_warning_state
                };


            let warning_overlay_active =
                active.fps_warning_state
                    != crate::fps_monitor::FpsWarningState::Normal;


            let overlay_should_display =
                subtitles
                    || warning_overlay_active;


            if overlay_should_display {

                let current_size =
                    (
                        width,
                        height,
                    );


                if current_size
                    != active.overlay_output_size
                    || warning_changed
                    || blink_changed
                    || active.subtitle_overlay.is_none()
                {
                    let warning_only_descriptor =
                        crate::construct_text_overlay::OverlayDescriptor::default();


                    let overlay_descriptor =
                        if subtitles {
                            &active.overlay_descriptor
                        } else {
                            &warning_only_descriptor
                        };


                    active.subtitle_overlay =
                        Some(
                            crate::display_overlay::OpenGlOverlay::new_with_fps_warning(
                                overlay_descriptor,
                                configured_fps,
                                overlay_warning_state,
                                subtitle_placement,
                                width,
                                height,
                            )?
                        );


                    active.overlay_output_size =
                        current_size;
                }


                if let Some(overlay) =
                    active.subtitle_overlay.as_ref()
                {
                    overlay.display(
                        width,
                        height,
                    );
                }

            } else {

                active.subtitle_overlay =
                    None;
            }


            let active_texture_selection =
                active.texture_manager
                    .active_specification_selection();


            let shader_information =
                crate::editor_layout::ShaderInformation {
                    filename:
                        information_path
                            .file_name()
                            .and_then(
                                |name| name.to_str()
                            )
                            .unwrap_or(
                                &active.shader_name
                            )
                            .to_string(),

                    folder:
                        information_path
                            .parent()
                            .unwrap_or_else(
                                || Path::new(".")
                            )
                            .display()
                            .to_string(),

                    shader_type:
                        describe_shader_type(
                            &information_path
                        ),

                    policies:
                        describe_policy_availability(
                            screensaver_policy_exists,
                            wallpaper_policy_exists,
                        ),

                    texture_usage:
                        if active.channel_usage
                            .uses_any_channel()
                        {
                            "Required".to_string()
                        } else {
                            "Not required".to_string()
                        },

                    status:
                        "Loaded and rendering".to_string(),
                };


            let editor_output =
                edit_window.display(
                    &window,
                    configured_fps,
                    animation_speed,
                    render_scale,
                    anti_aliasing_selection_from_method(
                        live_postprocess_profile
                            .anti_aliasing
                    ),
                    dithering_selection_from_level(
                        live_postprocess_profile
                            .dithering
                    ),
                    color_precision_selection_from_policy(
                        live_postprocess_profile
                            .color_precision
                    ),
                    active_texture_selection,
                    true,
                    active.channel_usage
                        .uses_any_channel(),
                    screensaver_target_available,
                    wallpaper_target_available,
                    target_restriction
                        == EditorTargetRestriction::WallpaperOnly,
                    target_restriction
                        == EditorTargetRestriction::ScreensaverOnly,
                    &recent_shader_paths,
                    Some(
                        &shader_information
                    ),
                    &policy_display_rows,
                    Some(&config),
                );

            if editor_output.control_configuration_save_requested {
                if let Some(control_configuration) =
                    editor_output.control_configuration.as_ref()
                {
                    match save_control_configuration(
                        control_configuration,
                    ) {
                        Ok(reloaded_config) => {
                            config =
                                reloaded_config;

                            policy_display_rows =
                                build_policy_display_rows(
                                    &config
                                );

                            edit_window.accept_control_configuration();

                            edit_window.set_status_message(
                                "Configuration saved."
                            );

                            log_information(
                                "[EDIT_SHADER] Configuration saved from Control Center"
                            );
                        }

                        Err(error) => {
                            edit_window.set_status_message(
                                "Configuration save failed."
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Unable to save configuration: {}",
                                    error,
                                )
                            );
                        }
                    }
                }
            }

            if !editor_output.window_open {
                break 'preview Ok(());
            }


            if let Some((
                target,
                index,
            )) =
                editor_output.control_single_recent_requested
            {
                if let Some(path) =
                    recent_shader_paths
                        .get(index)
                {
                    if let Some(filename) =
                        path.file_name()
                            .and_then(
                                |name| {
                                    name.to_str()
                                }
                            )
                    {
                        edit_window.set_control_single_filename(
                            target,
                            filename,
                        );

                        edit_window.set_status_message(
                            "Single shader selected."
                        );
                    }
                }
            }


            if let Some(target) =
                editor_output.control_single_browse_requested
            {
                let starting_directory =
                    match target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            crate::locate_paths::screensaver_shader_dir()
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            crate::locate_paths::wallpaper_shader_dir()
                        }
                    };

                let selected_path =
                    rfd::FileDialog::new()
                        .add_filter(
                            "GL shader files",
                            &[
                                "glsl",
                                "fs",
                            ],
                        )
                        .set_directory(
                            &starting_directory
                        )
                        .pick_file();

                if let Some(selected_path) =
                    selected_path
                {
                    if let Some(filename) =
                        selected_path
                            .file_name()
                            .and_then(
                                |name| {
                                    name.to_str()
                                }
                            )
                    {
                        edit_window.set_control_single_filename(
                            target,
                            filename,
                        );

                        promote_recent_shader_path(
                        &mut recent_shader_paths,
                        selected_path.clone(),
                    );

                        let _ =
                            save_recent_shader_paths(
                                &recent_shader_paths
                            );

                        edit_window.set_status_message(
                            "Single shader selected."
                        );
                    }
                } else {
                    edit_window.set_status_message(
                        "Shader selection canceled."
                    );
                }

                if let Err(error) =
                    restore_editor_fullscreen(
                        &mut window
                    )
                {
                    log_warning(
                        &format!(
                            "[EDIT_SHADER] Immediate fullscreen restoration failed: {}",
                            error,
                        )
                    );
                }

                fullscreen_restore_requested_at =
                    Some(
                        Instant::now()
                    );
            }


            if editor_output.clear_recent_files_requested {
                recent_shader_paths.clear();

                match save_recent_shader_paths(
                    &recent_shader_paths
                ) {
                    Ok(()) => {
                        edit_window.set_status_message(
                            "Recent shader-file history cleared."
                        );
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            format!(
                                "Recent files were cleared for this session, but the history file could not be updated: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to clear recent shader history: {}",
                                error,
                            )
                        );
                    }
                }
            }


            let recent_selected_path =
                editor_output.recent_shader_requested
                    .and_then(
                        |index| {
                            recent_shader_paths
                                .get(index)
                                .cloned()
                        }
                    );


            let policy_row_open_request =
                editor_output
                    .policy_row_command_requested
                    .as_ref()
                    .filter(
                        |(
                            _row,
                            command,
                        )| {
                            matches!(
                                *command,
                                crate::editor_layout::PolicyRowCommand::Edit
                                    | crate::editor_layout::PolicyRowCommand::RefreshShader
                            )
                        }
                    )
                    .cloned();


            if editor_output.browse_shader_requested
                || recent_selected_path.is_some()
                || policy_row_open_request.is_some()
            {
                let selected_path =
                    if let Some((
                        row,
                        _command,
                    )) =
                        policy_row_open_request
                            .as_ref()
                    {
                        Some(
                            resolve_policy_shader_path(
                                &config,
                                row.policy_target,
                                &row.filename,
                            )
                        )
                    } else if editor_output.browse_shader_requested {
                        let starting_directory =
                            active.path
                                .parent()
                                .unwrap_or_else(
                                    || Path::new(".")
                                );

                        let selected_path =
                            rfd::FileDialog::new()
                                .add_filter(
                                    "GL shader files",
                                    &[
                                        "glsl",
                                        "fs",
                                    ],
                                )
                                .set_directory(
                                    starting_directory
                                )
                                .pick_file();

                        if let Err(error) =
                            restore_editor_fullscreen(
                                &mut window
                            )
                        {
                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Immediate fullscreen restoration failed: {}",
                                    error,
                                )
                            );
                        }

                        fullscreen_restore_requested_at =
                            Some(
                                Instant::now()
                            );

                        selected_path
                    } else {
                        recent_selected_path
                    };


                let Some(selected_path) =
                    selected_path
                else {
                    edit_window.set_status_message(
                        "Shader loading canceled."
                    );

                    continue;
                };


                if !selected_path.is_file() {
                    if policy_row_open_request.is_some() {
                        edit_window.set_status_message(
                            format!(
                                "Policy shader file is unavailable: {}",
                                selected_path.display(),
                            )
                        );
                    } else {
                        recent_shader_paths.retain(
                            |path| {
                                path != &selected_path
                            }
                        );

                        let _ =
                            save_recent_shader_paths(
                                &recent_shader_paths
                            );

                        edit_window.set_status_message(
                            format!(
                                "Recent shader file no longer exists: {}",
                                selected_path.display(),
                            )
                        );
                    }

                    continue;
                };


                let selected_shader_name =
                    selected_path
                        .file_name()
                        .and_then(
                            |name| name.to_str()
                        )
                        .unwrap_or("")
                        .to_string();

                let mut new_screensaver_target_available =
                    target_restriction
                        == EditorTargetRestriction::Unrestricted
                    || target_shader_path(
                        crate::editor_layout::PolicyTarget::Screensaver,
                        &selected_shader_name,
                    )
                    .is_file();

                let mut new_wallpaper_target_available =
                    target_restriction
                        == EditorTargetRestriction::Unrestricted
                    || target_shader_path(
                        crate::editor_layout::PolicyTarget::Wallpaper,
                        &selected_shader_name,
                    )
                    .is_file();

                match target_restriction {
                    EditorTargetRestriction::WallpaperOnly => {
                        new_screensaver_target_available = false;
                    }
                    EditorTargetRestriction::ScreensaverOnly => {
                        new_wallpaper_target_available = false;
                    }
                    EditorTargetRestriction::Unrestricted => {}
                }

                let new_screensaver_policy_exists =
                    new_screensaver_target_available
                        && config.screensaver_policies
                        .iter()
                        .any(
                            |policy| {
                                policy_applies_to_path(
                                    policy,
                                    crate::editor_layout::PolicyTarget::Screensaver,
                                    &selected_path,
                                )
                            }
                        );

                let new_wallpaper_policy_exists =
                    new_wallpaper_target_available
                        && config.wallpaper_policies
                        .iter()
                        .any(
                            |policy| {
                                policy_applies_to_path(
                                    policy,
                                    crate::editor_layout::PolicyTarget::Wallpaper,
                                    &selected_path,
                                )
                            }
                        );

                let row_forced_target =
                    policy_row_open_request
                        .as_ref()
                        .map(
                            |(
                                row,
                                _command,
                            )| {
                                row.policy_target
                            }
                        );


                let new_editor_target =
                    if let Some(
                        row_forced_target
                    ) = row_forced_target
                    {
                        Some(
                            row_forced_target
                        )
                    } else if target_restriction
                        == EditorTargetRestriction::WallpaperOnly
                    {
                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        )
                    } else if target_restriction
                        == EditorTargetRestriction::ScreensaverOnly
                    {
                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        )
                    } else if new_wallpaper_policy_exists {
                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        )
                    } else if new_screensaver_policy_exists {
                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        )
                    } else {
                        None
                    };

                let (
                    new_global_rendered_fps,
                    new_fps_policy_entries,
                    new_texture_policy,
                    new_postprocess_policy,
                    new_animation_speed,
                    load_status,
                ) =
                    match new_editor_target {
                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        ) => {
                            (
                                config.wallpaper_fps_policy
                                    .global_rendered_fps,
                                config.wallpaper_fps_policy
                                    .fps_policy_entries
                                    .clone(),
                                config.wallpaper_texture_policy
                                    .clone(),
                                config.wallpaper_postprocess_policy
                                    .clone(),
                                config.wallpaper_speed_policy
                                    .animation_speed_for_shader(
                                        &selected_shader_name,
                                        command_line_animation_speed,
                                    ),
                                "Loaded shader with its existing Wallpaper policy."
                                    .to_string(),
                            )
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        ) => {
                            (
                                config.global_rendered_fps,
                                config.screensaver_fps_policy_entries
                                    .clone(),
                                config.texture_policy
                                    .clone(),
                                config.screensaver_postprocess_policy
                                    .clone(),
                                config.screensaver_speed_policy
                                    .animation_speed_for_shader(
                                        &selected_shader_name,
                                        command_line_animation_speed,
                                    ),
                                "Loaded shader with its existing Screensaver policy."
                                    .to_string(),
                            )
                        }

                        None => {
                            (
                                config.global_rendered_fps,
                                config.screensaver_fps_policy_entries
                                    .clone(),
                                config.texture_policy
                                    .clone(),
                                config.screensaver_postprocess_policy
                                    .clone(),
                                config.screensaver_speed_policy
                                    .animation_speed_for_shader(
                                        &selected_shader_name,
                                        command_line_animation_speed,
                                    ),
                                "Loaded shader using resolved defaults. Select a policy target to create a policy."
                                    .to_string(),
                            )
                        }
                    };

                let new_configured_fps =
                    resolve_preview_fps(
                        new_global_rendered_fps,
                        &new_fps_policy_entries,
                        command_line_fps,
                        &selected_shader_name,
                    );

                match load_active_shader(
                    &selected_path,
                    &new_texture_policy,
                    preview_selection,
                    subtitles,
                    subtitle_placement,
                    new_configured_fps,
                    new_animation_speed,
                    width,
                    height,
                ) {
                    Ok(mut replacement) => {
                        let new_live_postprocess_profile =
                            new_postprocess_policy
                                .profile_for_shader(
                                    &replacement.shader_name
                                );

                        postprocess.set_profile(
                            new_live_postprocess_profile
                        )?;

                        destroy_active_shader(
                            &mut active
                        );

                        std::mem::swap(
                            &mut active,
                            &mut replacement,
                        );

                        screensaver_target_available =
                            new_screensaver_target_available;

                        wallpaper_target_available =
                            new_wallpaper_target_available;

                        screensaver_policy_exists =
                            new_screensaver_policy_exists;

                        wallpaper_policy_exists =
                            new_wallpaper_policy_exists;

                        global_rendered_fps =
                            new_global_rendered_fps;

                        fps_policy_entries =
                            new_fps_policy_entries;

                        texture_policy =
                            new_texture_policy;

                        postprocess_policy =
                            new_postprocess_policy;

                        animation_speed =
                            new_animation_speed;

                        configured_fps =
                            new_configured_fps;

                        target_frame_time =
                            Duration::from_secs_f64(
                                1.0
                                    / configured_fps.max(1) as f64
                            );

                        live_postprocess_profile =
                            new_live_postprocess_profile;

                        render_scale =
                            live_postprocess_profile.render_scale;

                        information_path =
                            resolve_information_path(
                                &active.path,
                                &active.shader_name,
                                new_editor_target,
                            );

                        synchronize_overlay_texture_metadata(
                            &mut active
                        );

                        active.frame_times =
                            FrameTimeWindow::new();

                        active.fps_warning_state =
                            crate::fps_monitor::FpsWarningState::Normal;

                        active.fps_blink_visible =
                            true;

                        active.last_fps_blink =
                            Instant::now();

                        active.subtitle_overlay =
                            None;

                        edit_window.initialize_configuration(
                            configured_fps,
                            animation_speed,
                            render_scale,
                            new_editor_target,
                            anti_aliasing_selection_from_method(
                                live_postprocess_profile.anti_aliasing
                            ),
                            dithering_selection_from_level(
                                live_postprocess_profile.dithering
                            ),
                            color_precision_selection_from_policy(
                                live_postprocess_profile.color_precision
                            ),
                            active.texture_manager
                                .active_specification_selection(),
                            load_status,
                        );

                        log_information(
                            &format!(
                                "[EDIT_SHADER] Loaded shader from {}",
                                active.path.display(),
                            )
                        );

                        promote_recent_shader_path(
                            &mut recent_shader_paths,
                            active.path.clone(),
                        );

                        if let Err(error) =
                            save_recent_shader_paths(
                                &recent_shader_paths
                            )
                        {
                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Shader loaded, but recent-file history could not be saved: {}",
                                    error,
                                )
                            );

                            edit_window.set_status_message(
                                format!(
                                    "Shader loaded, but recent-file history could not be saved: {}",
                                    error,
                                )
                            );
                        }
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            format!(
                                "Unable to load shader: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to load '{}': {}",
                                selected_path.display(),
                                error,
                            )
                        );
                    }
                }

                continue;
            }


            if editor_output.refresh_shader_requested {
                let refresh_path =
                    active.path.clone();

                match load_active_shader(
                    &refresh_path,
                    &texture_policy,
                    preview_selection,
                    subtitles,
                    subtitle_placement,
                    configured_fps,
                    animation_speed,
                    width,
                    height,
                ) {
                    Ok(mut replacement) => {
                        destroy_active_shader(
                            &mut active
                        );

                        std::mem::swap(
                            &mut active,
                            &mut replacement,
                        );

                        information_path =
                            resolve_information_path(
                                &active.path,
                                &active.shader_name,
                                editor_output.policy_target,
                            );

                        edit_window.initialize_configuration(
                            configured_fps,
                            animation_speed,
                            render_scale,
                            editor_output.policy_target,
                            anti_aliasing_selection_from_method(
                                live_postprocess_profile.anti_aliasing
                            ),
                            dithering_selection_from_level(
                                live_postprocess_profile.dithering
                            ),
                            color_precision_selection_from_policy(
                                live_postprocess_profile.color_precision
                            ),
                            active.texture_manager
                                .active_specification_selection(),
                            format!(
                                "Refreshed shader from disk: {}",
                                active.shader_name,
                            ),
                        );

                        log_information(
                            &format!(
                                "[EDIT_SHADER] Refreshed shader from {}",
                                active.path.display(),
                            )
                        );
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            format!(
                                "Unable to refresh shader: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to refresh '{}': {}",
                                refresh_path.display(),
                                error,
                            )
                        );
                    }
                }

                continue;
            }


            if let Some(requested_target) =
                editor_output.policy_target_change_requested
            {
                let target_available =
                    match requested_target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            screensaver_target_available
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            wallpaper_target_available
                        }
                    };

                if !target_available {
                    edit_window.set_status_message(
                        match requested_target {
                            crate::editor_layout::PolicyTarget::Screensaver => {
                                "This shader cannot use a Screensaver policy because it does not exist in the screensavers folder."
                            }

                            crate::editor_layout::PolicyTarget::Wallpaper => {
                                "This shader cannot use a Wallpaper policy because it does not exist in the wallpapers folder."
                            }
                        }
                    );

                    continue;
                }

                let target_policy_exists =
                    match requested_target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            screensaver_policy_exists
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            wallpaper_policy_exists
                        }
                    };

                match requested_target {
                    crate::editor_layout::PolicyTarget::Screensaver => {
                        global_rendered_fps =
                            config.global_rendered_fps;

                        fps_policy_entries =
                            config.screensaver_fps_policy_entries
                                .clone();

                        texture_policy =
                            config.texture_policy
                                .clone();

                        postprocess_policy =
                            config.screensaver_postprocess_policy
                                .clone();

                        animation_speed =
                            config.screensaver_speed_policy
                                .animation_speed_for_shader(
                                    &active.shader_name,
                                    command_line_animation_speed,
                                );
                    }

                    crate::editor_layout::PolicyTarget::Wallpaper => {
                        global_rendered_fps =
                            config.wallpaper_fps_policy
                                .global_rendered_fps;

                        fps_policy_entries =
                            config.wallpaper_fps_policy
                                .fps_policy_entries
                                .clone();

                        texture_policy =
                            config.wallpaper_texture_policy
                                .clone();

                        postprocess_policy =
                            config.wallpaper_postprocess_policy
                                .clone();

                        animation_speed =
                            config.wallpaper_speed_policy
                                .animation_speed_for_shader(
                                    &active.shader_name,
                                    command_line_animation_speed,
                                );
                    }
                }

                configured_fps =
                    resolve_preview_fps(
                        global_rendered_fps,
                        &fps_policy_entries,
                        command_line_fps,
                        &active.shader_name,
                    );

                target_frame_time =
                    Duration::from_secs_f64(
                        1.0
                            / configured_fps.max(1) as f64
                    );

                live_postprocess_profile =
                    postprocess_policy.profile_for_shader(
                        &active.shader_name
                    );

                postprocess.set_profile(
                    live_postprocess_profile
                )?;

                render_scale =
                    live_postprocess_profile.render_scale;

                active.texture_manager
                    .delete_all();

                active.texture_manager =
                    crate::manage_textures::TextureManager::new(
                        texture_policy.clone()
                    );

                active.texture_manager
                    .prepare_for_shader_with_selection(
                        &active.shader_name,
                        active.channel_usage,
                        crate::manage_textures::PreviewTextureSelection {
                            texture:
                                None,

                            palette:
                                None,
                        },
                    )?;

                active.texture_manager
                    .configure_program(
                        active.program
                    );

                synchronize_overlay_texture_metadata(
                    &mut active
                );

                active.frame_times =
                    FrameTimeWindow::new();

                active.fps_warning_state =
                    crate::fps_monitor::FpsWarningState::Normal;

                active.fps_blink_visible =
                    true;

                active.last_fps_blink =
                    Instant::now();

                active.overlay_descriptor.shader =
                    Some(
                        format!(
                            "{} | {}",
                            active.shader_name,
                            format_animation_speed(
                                animation_speed
                            ),
                        )
                    );

                active.subtitle_overlay =
                    None;

                information_path =
                    resolve_information_path(
                        &active.path,
                        &active.shader_name,
                        Some(
                            requested_target
                        ),
                    );


                let target_name =
                    match requested_target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            "Screensaver"
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            "Wallpaper"
                        }
                    };

                let status_message =
                    if target_policy_exists {
                        format!(
                            "Loaded existing {} policy for this shader.",
                            target_name,
                        )
                    } else {
                        format!(
                            "No {} policy exists. Loaded {} defaults.",
                            target_name,
                            target_name,
                        )
                    };

                edit_window.initialize_configuration(
                    configured_fps,
                    animation_speed,
                    render_scale,
                    Some(
                        requested_target
                    ),
                    anti_aliasing_selection_from_method(
                        live_postprocess_profile.anti_aliasing
                    ),
                    dithering_selection_from_level(
                        live_postprocess_profile.dithering
                    ),
                    color_precision_selection_from_policy(
                        live_postprocess_profile.color_precision
                    ),
                    active.texture_manager
                        .active_specification_selection(),
                    status_message,
                );

                log_information(
                    &format!(
                        "[EDIT_SHADER] Policy target switched to {} ({})",
                        target_name,
                        if target_policy_exists {
                            "existing policy"
                        } else {
                            "resolved defaults"
                        },
                    )
                );

                continue;
            }


            let selected_fps =
                editor_output.fps;

            let selected_animation_speed =
                editor_output.animation_speed;

            let selected_render_scale =
                editor_output.render_scale;

            let selected_policy_target =
                editor_output.policy_target;

            let selected_texture =
                editor_output.texture;

            let selected_palette =
                editor_output.palette;

            let selected_primitive_count =
                editor_output.primitive_count;

            let selected_anti_aliasing =
                editor_output.anti_aliasing;

            let selected_dithering =
                editor_output.dithering;

            let selected_color_precision =
                editor_output.color_precision;


            if selected_fps
                != configured_fps
            {
                configured_fps =
                    selected_fps;

                target_frame_time =
                    Duration::from_secs_f64(
                        1.0
                            / configured_fps.max(1) as f64
                    );

                active.frame_times =
                    FrameTimeWindow::new();

                active.fps_warning_state =
                    crate::fps_monitor::FpsWarningState::Normal;

                active.fps_blink_visible =
                    true;

                active.last_fps_blink =
                    Instant::now();

                active.subtitle_overlay =
                    None;

                log_information(
                    &format!(
                        "[EDIT_SHADER] Live FPS target changed to {}",
                        configured_fps,
                    )
                );
            }


            if (
                selected_animation_speed
                    - animation_speed
            )
                .abs()
                > f32::EPSILON
            {
                animation_speed =
                    selected_animation_speed;

                active.overlay_descriptor.shader =
                    Some(
                        format!(
                            "{} | {}",
                            active.shader_name,
                            format_animation_speed(
                                animation_speed
                            ),
                        )
                    );

                active.subtitle_overlay =
                    None;

                log_information(
                    &format!(
                        "[EDIT_SHADER] Live animation speed changed to {:.2}x",
                        animation_speed,
                    )
                );
            }


            let selected_anti_aliasing_method =
                anti_aliasing_method_from_selection(
                    selected_anti_aliasing
                );

            let selected_dithering_level =
                dithering_level_from_selection(
                    selected_dithering
                );

            let selected_color_precision_policy =
                color_precision_policy_from_selection(
                    selected_color_precision
                );


            if (
                selected_render_scale
                    - live_postprocess_profile.render_scale
            )
                .abs()
                > f32::EPSILON
                || selected_anti_aliasing_method
                    != live_postprocess_profile.anti_aliasing
                || selected_dithering_level
                    != live_postprocess_profile.dithering
                || selected_color_precision_policy
                    != live_postprocess_profile.color_precision
            {
                live_postprocess_profile.render_scale =
                    selected_render_scale;

                live_postprocess_profile.anti_aliasing =
                    selected_anti_aliasing_method;

                live_postprocess_profile.dithering =
                    selected_dithering_level;

                live_postprocess_profile.color_precision =
                    selected_color_precision_policy;

                postprocess.set_profile(
                    live_postprocess_profile
                )?;

                render_scale =
                    live_postprocess_profile.render_scale;

                active.frame_times =
                    FrameTimeWindow::new();

                active.fps_warning_state =
                    crate::fps_monitor::FpsWarningState::Normal;

                active.fps_blink_visible =
                    true;

                active.last_fps_blink =
                    Instant::now();

                active.subtitle_overlay =
                    None;

                log_information(
                    &format!(
                        "[EDIT_SHADER] Live post-processing changed: anti_aliasing={}, dithering={}, color_precision={}, render_scale={:.2}",
                        live_postprocess_profile
                            .anti_aliasing
                            .name(),
                        live_postprocess_profile
                            .dithering
                            .name(),
                        live_postprocess_profile
                            .color_precision
                            .name(),
                        render_scale,
                    )
                );
            }


            if active.channel_usage
                .uses_any_channel()
            {
                let selected_specification =
                    TextureSpecification {
                        family:
                            selected_texture.family(),

                        requested_primitive_count:
                            selected_primitive_count as usize,

                        count_was_explicit:
                            true,
                    };

                let current_selection =
                    active.texture_manager
                        .active_specification_selection();

                let texture_changed =
                    current_selection
                        .map(
                            |(
                                specification,
                                palette,
                            )| {
                                specification.family
                                    != selected_specification.family
                                    || specification.requested_primitive_count
                                        != selected_specification.requested_primitive_count
                                    || palette
                                        != selected_palette.palette()
                            }
                        )
                        .unwrap_or(
                            true
                        );

                if texture_changed {
                    active.texture_manager
                        .prepare_for_shader_with_selection(
                            &active.shader_name,
                            active.channel_usage,
                            crate::manage_textures::PreviewTextureSelection {
                                texture:
                                    Some(
                                        crate::manage_textures::PreviewSelectionValue::Specific(
                                            selected_specification
                                        )
                                    ),

                                palette:
                                    Some(
                                        crate::manage_textures::PreviewSelectionValue::Specific(
                                            selected_palette.palette()
                                        )
                                    ),
                            },
                        )?;

                    active.texture_manager
                        .configure_program(
                            active.program
                        );

                    synchronize_overlay_texture_metadata(
                        &mut active
                    );

                    active.subtitle_overlay =
                        None;

                    log_information(
                        &format!(
                            "[EDIT_SHADER] Live procedural texture changed to {}:{} with palette {}",
                            selected_texture.name(),
                            selected_primitive_count,
                            selected_palette.name(),
                        )
                    );
                }
            }


            if editor_output.save_requested {
                let Some(policy_target) =
                    selected_policy_target
                else {
                    edit_window.set_status_message(
                        "Select a policy target before saving"
                    );

                    continue;
                };

                let selected_target_available =
                    match policy_target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            screensaver_target_available
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            wallpaper_target_available
                        }
                    };

                if !selected_target_available {
                    edit_window.set_status_message(
                        "The selected policy target is unavailable because the shader file is missing from its target folder."
                    );

                    continue;
                }

                let texture_specification =
                    if active.channel_usage
                        .uses_any_channel()
                    {
                        Some(
                            format!(
                                "{}:{}",
                                selected_texture.name(),
                                selected_primitive_count,
                            )
                        )
                    } else {
                        None
                    };

                let palette_name =
                    if active.channel_usage
                        .uses_any_channel()
                    {
                        Some(
                            selected_palette
                                .name()
                                .to_string()
                        )
                    } else {
                        None
                    };

                let properties =
                    crate::manage_policies::PolicyDefinition {
                        texture:
                            texture_specification,

                        palette:
                            palette_name,

                        fps:
                            Some(
                                configured_fps
                            ),

                        speed:
                            Some(
                                animation_speed
                            ),

                        render_scale:
                            Some(
                                render_scale
                            ),

                        anti_aliasing:
                            Some(
                                live_postprocess_profile
                                    .anti_aliasing
                                    .name()
                                    .to_ascii_lowercase()
                            ),

                        dithering:
                            Some(
                                live_postprocess_profile
                                    .dithering
                                    .name()
                                    .to_ascii_lowercase()
                            ),

                        color_precision:
                            Some(
                                live_postprocess_profile
                                    .color_precision
                                    .name()
                                    .to_string()
                            ),
                    };

                let manage_target =
                    match policy_target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            crate::manage_policies::PolicyTarget::Screensaver
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            crate::manage_policies::PolicyTarget::Wallpaper
                        }
                    };

                let config_path =
                    crate::locate_paths::config_path();

                let managed_target_path =
                    target_shader_path(
                        policy_target,
                        &active.shader_name,
                    );

                let external_source_path =
                    if paths_refer_to_same_shader(
                        &active.path,
                        &managed_target_path,
                    ) {
                        None
                    } else {
                        Some(
                            active.path.as_path()
                        )
                    };

                let save_result =
                    if crate::manage_policies::policy_exists(
                        &config_path,
                        manage_target,
                        &active.shader_name,
                    )? {
                        crate::manage_policies::replace_policy_with_source_path(
                            &config_path,
                            manage_target,
                            &active.shader_name,
                            properties,
                            external_source_path,
                        )
                    } else {
                        crate::manage_policies::add_policy_with_source_path(
                            &config_path,
                            manage_target,
                            &active.shader_name,
                            properties,
                            external_source_path,
                        )
                    };

                match save_result {
                    Ok(()) => {
                        match crate::load_config::load_config(
                            &config_path
                        ) {
                            Ok(reloaded_config) => {
                                config =
                                    reloaded_config.config;

                                policy_display_rows =
                                    build_policy_display_rows(
                                        &config
                                    );

                                screensaver_policy_exists =
                                    screensaver_target_available
                                        && config.screensaver_policies
                                        .iter()
                                        .any(
                                            |policy| {
                                                policy.shader
                                                    .eq_ignore_ascii_case(
                                                        &active.shader_name
                                                    )
                                            }
                                        );

                                wallpaper_policy_exists =
                                    wallpaper_target_available
                                        && config.wallpaper_policies
                                        .iter()
                                        .any(
                                            |policy| {
                                                policy.shader
                                                    .eq_ignore_ascii_case(
                                                        &active.shader_name
                                                    )
                                            }
                                        );
                            }

                            Err(error) => {
                                log_warning(
                                    &format!(
                                        "[EDIT_SHADER] Policy saved, but configuration reload failed: {}",
                                        error,
                                    )
                                );
                            }
                        }

                        edit_window.accept_current_configuration();

                        edit_window.set_status_message(
                            format!(
                                "Policy saved for {}",
                                manage_target.name(),
                            )
                        );

                        log_information(
                            &format!(
                                "[EDIT_SHADER] Saved {} policy for {}",
                                manage_target.name(),
                                active.shader_name,
                            )
                        );
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            format!(
                                "Unable to save policy: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to save policy for {}: {}",
                                active.shader_name,
                                error,
                            )
                        );
                    }
                }
            }


            if let Some((
                row,
                command,
            )) =
                editor_output
                    .policy_row_command_requested
                    .as_ref()
            {
                match command {
                    crate::editor_layout::PolicyRowCommand::DeletePolicy => {
                        let manage_target =
                            match row.policy_target {
                                crate::editor_layout::PolicyTarget::Screensaver => {
                                    crate::manage_policies::PolicyTarget::Screensaver
                                }

                                crate::editor_layout::PolicyTarget::Wallpaper => {
                                    crate::manage_policies::PolicyTarget::Wallpaper
                                }
                            };

                        let config_path =
                            crate::locate_paths::config_path();

                        match crate::manage_policies::delete_policy(
                            &config_path,
                            manage_target,
                            &row.filename,
                        ) {
                            Ok(()) => {
                                match crate::load_config::load_config(
                                    &config_path
                                ) {
                                    Ok(reloaded_config) => {
                                        config =
                                            reloaded_config.config;

                                        policy_display_rows =
                                            build_policy_display_rows(
                                                &config
                                            );

                                        screensaver_policy_exists =
                                            screensaver_target_available
                                                && config.screensaver_policies
                                                .iter()
                                                .any(
                                                    |policy| {
                                                        policy.shader
                                                            .eq_ignore_ascii_case(
                                                                &active.shader_name
                                                            )
                                                    }
                                                );

                                        wallpaper_policy_exists =
                                            wallpaper_target_available
                                                && config.wallpaper_policies
                                                .iter()
                                                .any(
                                                    |policy| {
                                                        policy.shader
                                                            .eq_ignore_ascii_case(
                                                                &active.shader_name
                                                            )
                                                    }
                                                );
                                    }

                                    Err(error) => {
                                        log_warning(
                                            &format!(
                                                "[EDIT_SHADER] Policy deleted, but configuration reload failed: {}",
                                                error,
                                            )
                                        );
                                    }
                                }

                                edit_window.set_status_message(
                                    format!(
                                        "{} policy deleted for {}",
                                        manage_target.name(),
                                        row.filename,
                                    )
                                );

                                log_information(
                                    &format!(
                                        "[EDIT_SHADER] Deleted {} policy for {}",
                                        manage_target.name(),
                                        row.filename,
                                    )
                                );
                            }

                            Err(error) => {
                                edit_window.set_status_message(
                                    format!(
                                        "Unable to delete policy: {}",
                                        error,
                                    )
                                );

                                log_warning(
                                    &format!(
                                        "[EDIT_SHADER] Unable to delete {} policy for {}: {}",
                                        manage_target.name(),
                                        row.filename,
                                        error,
                                    )
                                );
                            }
                        }

                        continue;
                    }

                    crate::editor_layout::PolicyRowCommand::DeleteShader => {
                        let manage_target =
                            match row.policy_target {
                                crate::editor_layout::PolicyTarget::Screensaver => {
                                    crate::manage_policies::PolicyTarget::Screensaver
                                }

                                crate::editor_layout::PolicyTarget::Wallpaper => {
                                    crate::manage_policies::PolicyTarget::Wallpaper
                                }
                            };

                        let shader_path =
                            resolve_policy_shader_path(
                                &config,
                                row.policy_target,
                                &row.filename,
                            );

                        let config_path =
                            crate::locate_paths::config_path();

                        let policy_delete_result =
                            crate::manage_policies::delete_policy(
                                &config_path,
                                manage_target,
                                &row.filename,
                            );

                        match policy_delete_result {
                            Ok(()) => {
                                match std::fs::remove_file(
                                    &shader_path
                                ) {
                                    Ok(()) => {
                                        match crate::load_config::load_config(
                                            &config_path
                                        ) {
                                            Ok(reloaded_config) => {
                                                config =
                                                    reloaded_config.config;

                                                policy_display_rows =
                                                    build_policy_display_rows(
                                                        &config
                                                    );
                                            }

                                            Err(error) => {
                                                log_warning(
                                                    &format!(
                                                        "[EDIT_SHADER] Shader/policy deleted, but configuration reload failed: {}",
                                                        error,
                                                    )
                                                );
                                            }
                                        }

                                        edit_window.set_status_message(
                                            format!(
                                                "{} shader and associated {} policy deleted: {}",
                                                manage_target.name(),
                                                manage_target.name(),
                                                row.filename,
                                            )
                                        );

                                        log_information(
                                            &format!(
                                                "[EDIT_SHADER] Deleted {} shader {} and its policy",
                                                manage_target.name(),
                                                shader_path.display(),
                                            )
                                        );
                                    }

                                    Err(error) => {
                                        edit_window.set_status_message(
                                            format!(
                                                "{} policy was deleted, but the shader file could not be deleted: {}",
                                                manage_target.name(),
                                                error,
                                            )
                                        );

                                        log_warning(
                                            &format!(
                                                "[EDIT_SHADER] Deleted {} policy for {}, but failed to delete shader file {}: {}",
                                                manage_target.name(),
                                                row.filename,
                                                shader_path.display(),
                                                error,
                                            )
                                        );
                                    }
                                }
                            }

                            Err(error) => {
                                edit_window.set_status_message(
                                    format!(
                                        "Shader was not deleted because its associated policy could not be deleted: {}",
                                        error,
                                    )
                                );

                                log_warning(
                                    &format!(
                                        "[EDIT_SHADER] Refusing to delete shader {} because {} policy deletion failed: {}",
                                        shader_path.display(),
                                        manage_target.name(),
                                        error,
                                    )
                                );
                            }
                        }

                        continue;
                    }

                    crate::editor_layout::PolicyRowCommand::Edit
                    | crate::editor_layout::PolicyRowCommand::RefreshShader => {
                        // These commands are handled by the shader-load branch
                        // above so the row target is loaded explicitly.
                    }
                }
            }


            if editor_output.delete_requested {
                edit_window.set_status_message(
                    "Delete Shader is available from the Policies row context menu."
                );
            }


            window.gl_swap_window();


            active.frame =
                active.frame.saturating_add(
                    1
                );


            active.previous_frame =
                Instant::now();


            let render_elapsed =
                frame_start.elapsed();


            if render_elapsed
                < target_frame_time
            {
                std::thread::sleep(
                    target_frame_time
                        - render_elapsed
                );
            }
        };


    destroy_active_shader(
        &mut active
    );


    unsafe {
        if vao
            != 0
        {
            gl::DeleteVertexArrays(
                1,
                &vao,
            );
        }
    }


    drop(
        postprocess
    );


    edit_window.destroy();


    drop(
        gl_context
    );


    log_information(
        "[EDIT_SHADER] Edit session closed"
    );


    result
}




fn resolved_shader_policy_path(
    policy: &crate::load_config::ShaderPolicy,
    target: crate::editor_layout::PolicyTarget,
) -> PathBuf {

    policy.source_path
        .clone()
        .unwrap_or_else(
            || {
                target_shader_path(
                    target,
                    &policy.shader,
                )
            }
        )
}


fn resolve_policy_shader_path(
    config: &crate::load_config::Config,
    target: crate::editor_layout::PolicyTarget,
    shader_name: &str,
) -> PathBuf {

    let policies =
        match target {
            crate::editor_layout::PolicyTarget::Screensaver => {
                &config.screensaver_policies
            }

            crate::editor_layout::PolicyTarget::Wallpaper => {
                &config.wallpaper_policies
            }
        };


    policies
        .iter()
        .find(
            |policy| {
                policy.shader
                    .eq_ignore_ascii_case(
                        shader_name
                    )
            }
        )
        .map(
            |policy| {
                resolved_shader_policy_path(
                    policy,
                    target,
                )
            }
        )
        .unwrap_or_else(
            || {
                target_shader_path(
                    target,
                    shader_name,
                )
            }
        )
}


fn policy_applies_to_path(
    policy: &crate::load_config::ShaderPolicy,
    target: crate::editor_layout::PolicyTarget,
    loaded_path: &Path,
) -> bool {

    let Some(loaded_name) =
        loaded_path
            .file_name()
            .and_then(
                |name| {
                    name.to_str()
                }
            )
    else {
        return false;
    };


    if !policy.shader
        .eq_ignore_ascii_case(
            loaded_name
        )
    {
        return false;
    }


    paths_refer_to_same_shader(
        loaded_path,
        &resolved_shader_policy_path(
            policy,
            target,
        ),
    )
}

fn paths_refer_to_same_shader(
    loaded_path: &Path,
    managed_path: &Path,
) -> bool {

    match (
        loaded_path.canonicalize(),
        managed_path.canonicalize(),
    ) {
        (
            Ok(loaded_path),
            Ok(managed_path),
        ) => {
            loaded_path
                == managed_path
        }

        _ => {
            loaded_path
                == managed_path
        }
    }
}


fn target_shader_path(
    target: crate::editor_layout::PolicyTarget,
    shader_name: &str,
) -> PathBuf {
    match target {
        crate::editor_layout::PolicyTarget::Screensaver => {
            crate::locate_paths::screensaver_shader_dir()
                .join(shader_name)
        }

        crate::editor_layout::PolicyTarget::Wallpaper => {
            crate::locate_paths::wallpaper_shader_dir()
                .join(shader_name)
        }
    }
}


fn resolve_information_path(
    loaded_path: &Path,
    shader_name: &str,
    policy_target: Option<crate::editor_layout::PolicyTarget>,
) -> PathBuf {

    let target_path =
        match policy_target {
            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ) => {
                crate::locate_paths::screensaver_shader_dir()
                    .join(
                        shader_name
                    )
            }

            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ) => {
                crate::locate_paths::wallpaper_shader_dir()
                    .join(
                        shader_name
                    )
            }

            None => {
                return loaded_path.to_path_buf();
            }
        };


    if loaded_path.is_file() {
        loaded_path.to_path_buf()
    } else if target_path.is_file() {
        target_path
    } else {
        loaded_path.to_path_buf()
    }
}


fn describe_policy_availability(
    screensaver_policy_exists: bool,
    wallpaper_policy_exists: bool,
) -> String {
    match (
        screensaver_policy_exists,
        wallpaper_policy_exists,
    ) {
        (
            true,
            true,
        ) => {
            "Screensaver + Wallpaper"
                .to_string()
        }

        (
            true,
            false,
        ) => {
            "Screensaver"
                .to_string()
        }

        (
            false,
            true,
        ) => {
            "Wallpaper"
                .to_string()
        }

        (
            false,
            false,
        ) => {
            "None"
                .to_string()
        }
    }
}


fn describe_shader_type(
    path: &Path,
) -> String {
    let extension =
        path.extension()
            .and_then(
                |value| {
                    value.to_str()
                }
            )
            .unwrap_or("")
            .to_ascii_lowercase();

    if extension == "fs" {
        return "ISF".to_string();
    }

    let source =
        std::fs::read_to_string(
            path
        )
        .unwrap_or_default();

    if source.contains(
        "\"ISFVSN\""
    ) {
        "ISF".to_string()
    } else if source.contains(
        "mainImage"
    ) {
        "ShaderToy".to_string()
    } else {
        "Native GLSL".to_string()
    }
}


fn build_policy_display_rows(
    config: &crate::load_config::Config,
) -> Vec<crate::editor_layout::PolicyDisplayRow> {

    let mut rows =
        Vec::with_capacity(
            config.screensaver_policies.len()
                + config.wallpaper_policies.len()
        );


    rows.extend(
        config.screensaver_policies
            .iter()
            .map(
                |policy| {
                    let resolved_path =
                        resolved_shader_policy_path(
                            policy,
                            crate::editor_layout::PolicyTarget::Screensaver,
                        );


                    crate::editor_layout::PolicyDisplayRow {
                        filename:
                            policy.shader.clone(),

                        full_path:
                            resolved_path
                                .display()
                                .to_string(),

                        accessible:
                            resolved_path.is_file(),

                        texture:
                            policy.shader_texture.is_some(),

                        policy_target:
                            crate::editor_layout::PolicyTarget::Screensaver,
                    }
                }
            )
    );


    rows.extend(
        config.wallpaper_policies
            .iter()
            .map(
                |policy| {
                    let resolved_path =
                        resolved_shader_policy_path(
                            policy,
                            crate::editor_layout::PolicyTarget::Wallpaper,
                        );


                    crate::editor_layout::PolicyDisplayRow {
                        filename:
                            policy.shader.clone(),

                        full_path:
                            resolved_path
                                .display()
                                .to_string(),

                        accessible:
                            resolved_path.is_file(),

                        texture:
                            policy.shader_texture.is_some(),

                        policy_target:
                            crate::editor_layout::PolicyTarget::Wallpaper,
                    }
                }
            )
    );


    rows
}

fn resolve_preview_fps(
    global_rendered_fps: u32,
    fps_policy_entries:
        &[
            crate::load_config::FpsPolicyEntry
        ],
    command_line_fps: Option<u32>,
    shader_name: &str,
) -> u32 {

    if let Some(fps) =
        command_line_fps
    {
        return fps.max(
            1
        );
    }


    fps_policy_entries
        .iter()
        .find(
            |fps_policy_entry| {
                fps_policy_entry
                    .shader
                    .eq_ignore_ascii_case(
                        shader_name
                    )
            }
        )
        .map(
            |fps_policy_entry| {
                fps_policy_entry.rendered_fps
            }
        )
        .unwrap_or(
            global_rendered_fps
        )
        .max(
            1
        )
}


fn load_first_usable_shader(
    paths: &[PathBuf],
    start_index: usize,
    texture_policy:
        &crate::load_config::TexturePolicy,
    preview_selection:
        crate::manage_textures::PreviewTextureSelection,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    global_rendered_fps: u32,
    fps_policy_entries:
        &[
            crate::load_config::FpsPolicyEntry
        ],
    command_line_fps: Option<u32>,
    animation_speed: f32,
    output_width: u32,
    output_height: u32,
) -> Result<
    (
        ActivePreviewShader,
        usize,
    ),
    String,
> {

    for offset in
        0..paths.len()
    {
        let index =
            (
                start_index
                    + offset
            )
                % paths.len();


        let path =
            &paths[
                index
            ];


        let shader_name =
            path.file_name()
                .and_then(
                    |value| {
                        value.to_str()
                    }
                )
                .unwrap_or_default();


        let configured_fps =
            resolve_preview_fps(
                global_rendered_fps,
                fps_policy_entries,
                command_line_fps,
                shader_name,
            );


        match load_active_shader(
            path,
            texture_policy,
            preview_selection,
            subtitles,
            subtitle_placement,
            configured_fps,
            animation_speed,
            output_width,
            output_height,
        ) {

            Ok(shader) => {
                return Ok(
                    (
                        shader,
                        index,
                    )
                );
            }

            Err(error) => {

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Skipping '{}': {}",
                        path.display(),
                        error,
                    )
                );
            }
        }
    }


    Err(
        "The selected shader could not be loaded for editing"
            .to_string()
    )
}


fn load_active_shader(
    path: &Path,
    texture_policy:
        &crate::load_config::TexturePolicy,
    preview_selection:
        crate::manage_textures::PreviewTextureSelection,
    subtitles: bool,
    subtitle_placement:
        crate::parse_subtitle_placement::SubtitlePlacement,
    configured_fps: u32,
    animation_speed: f32,
    output_width: u32,
    output_height: u32,
) -> Result<
    ActivePreviewShader,
    String,
> {

    let loaded =
        crate::load_shader::load_shader_for_preview(
            path
        );


    let (
        source,
        shader_name,
        channel_usage,
        shader_inputs,
    ) =
        match loaded {

            crate::load_shader::ShaderLoadResult::Ready {
                source,
                shader_name,
                channel_usage,
                shader_inputs,
                ..
            } => {
                (
                    source,
                    shader_name,
                    channel_usage,
                    shader_inputs,
                )
            }

            crate::load_shader::ShaderLoadResult::Rejected {
                reasons,
                ..
            } => {
                return Err(
                    format!(
                        "rejected: {}",
                        reasons.join(
                            "; "
                        ),
                    )
                );
            }

            crate::load_shader::ShaderLoadResult::Unavailable {
                error,
                ..
            } => {
                return Err(
                    format!(
                        "unavailable: {}",
                        error,
                    )
                );
            }
        };


    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            &source,
        )?;


    let mut texture_manager =
        crate::manage_textures::TextureManager::new(
            texture_policy.clone()
        );


    if let Err(error) =
        texture_manager.prepare_for_shader_with_selection(
            &shader_name,
            channel_usage,
            preview_selection,
        )
    {
        unsafe {
            gl::DeleteProgram(
                program
            );
        }


        return Err(
            error
        );
    }


    texture_manager.configure_program(
        program
    );


    let (
        texture,
        palette,
    ) =
        texture_manager
            .active_specification_selection()
            .map(
                |(
                    specification,
                    palette,
                )| {
                    let texture_name =
                        specification.display_name();


                    let texture_description =
                        if specification.count_was_explicit {

                            texture_name

                        } else {

                            format!(
                                "{} ({})",
                                texture_name,
                                specification.requested_primitive_count,
                            )
                        };


                    (
                        Some(
                            texture_description
                        ),
                        Some(
                            palette.to_string()
                        ),
                    )
                }
            )
            .unwrap_or(
                (
                    None,
                    None,
                )
            );


    let overlay_descriptor =
        crate::construct_text_overlay::OverlayDescriptor {
            shader:
                Some(
                    format!(
                        "{} | {}",
                        shader_name,
                        format_animation_speed(
                            animation_speed
                        ),
                    )
                ),

            texture,

            palette,
        };


    let subtitle_overlay =
        if subtitles {

            Some(
                crate::display_overlay::OpenGlOverlay::new_with_fps_warning(
                    &overlay_descriptor,
                    configured_fps,
                    crate::fps_monitor::FpsWarningState::Normal,
                    subtitle_placement,
                    output_width,
                    output_height,
                )?
            )

        } else {

            None
        };


    log_information(
        &format!(
            "[EDIT_SHADER] Active shader: {}",
            path.display(),
        )
    );


    Ok(
        ActivePreviewShader {
            path:
                path.to_path_buf(),
            shader_name,
            program,
            channel_usage,
            shader_inputs,
            texture_manager,
            overlay_descriptor,
            subtitle_overlay,
            overlay_output_size:
                (
                    output_width,
                    output_height,
                ),
            fps_warning_state:
                crate::fps_monitor::FpsWarningState::Normal,
            fps_blink_visible:
                true,
            last_fps_blink:
                Instant::now(),
            frame_times:
                FrameTimeWindow::new(),
            start_time:
                Instant::now(),
            previous_frame:
                Instant::now(),
            frame:
                0,
        }
    )
}



fn synchronize_overlay_texture_metadata(
    active: &mut ActivePreviewShader,
) {

    let (
        texture,
        palette,
    ) =
        active.texture_manager
            .active_specification_selection()
            .map(
                |(
                    specification,
                    palette,
                )| {
                    let texture_name =
                        specification.display_name();


                    let texture_description =
                        if specification.count_was_explicit {

                            texture_name

                        } else {

                            format!(
                                "{} ({})",
                                texture_name,
                                specification.requested_primitive_count,
                            )
                        };


                    (
                        Some(
                            texture_description
                        ),
                        Some(
                            palette.to_string()
                        ),
                    )
                }
            )
            .unwrap_or(
                (
                    None,
                    None,
                )
            );


    active.overlay_descriptor.texture =
        texture;

    active.overlay_descriptor.palette =
        palette;
}

fn format_animation_speed(
    speed: f32,
) -> String {

    if speed.fract()
        == 0.0
    {
        format!(
            "×{speed:.1}"
        )
    } else {
        format!(
            "×{speed}"
        )
    }
}


fn destroy_active_shader(
    active: &mut ActivePreviewShader,
) {

    active.subtitle_overlay =
        None;


    active.texture_manager
        .delete_all();


    unsafe {
        if active.program
            != 0
        {
            gl::DeleteProgram(
                active.program
            );


            active.program =
                0;
        }
    }
}


fn load_recent_shader_paths() -> Vec<PathBuf> {
    let history_path =
        crate::locate_paths::recent_shader_history_path();

    let Ok(text) =
        std::fs::read_to_string(
            &history_path
        )
    else {
        return Vec::new();
    };


    let Ok(stored_paths) =
        serde_json::from_str::<Vec<String>>(
            &text
        )
    else {
        log_warning(
            &format!(
                "[EDIT_SHADER] Ignoring invalid recent shader history at {}",
                history_path.display(),
            )
        );

        return Vec::new();
    };


    let mut recent_paths =
        Vec::new();


    for stored_path in stored_paths {
        let path =
            PathBuf::from(
                stored_path
            );

        if !path.is_file()
            || !is_supported_shader_path(
                &path
            )
            || recent_paths.iter().any(
                |existing| {
                    existing == &path
                }
            )
        {
            continue;
        }

        recent_paths.push(
            path
        );

        if recent_paths.len()
            >= RECENT_SHADER_LIMIT
        {
            break;
        }
    }


    let _ =
        save_recent_shader_paths(
            &recent_paths
        );


    recent_paths
}


fn save_recent_shader_paths(
    recent_paths: &[PathBuf],
) -> Result<(), String> {

    let history_path =
        crate::locate_paths::recent_shader_history_path();


    if let Some(parent) =
        history_path.parent()
    {
        std::fs::create_dir_all(
            parent
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create recent-history folder {}: {}",
                    parent.display(),
                    error,
                )
            }
        )?;
    }


    let stored_paths:
        Vec<String> =
        recent_paths
            .iter()
            .take(
                RECENT_SHADER_LIMIT
            )
            .map(
                |path| {
                    path.to_string_lossy()
                        .into_owned()
                }
            )
            .collect();


    let serialized =
        serde_json::to_string_pretty(
            &stored_paths
        )
        .map_err(
            |error| {
                format!(
                    "Unable to serialize recent shader history: {}",
                    error,
                )
            }
        )?;


    std::fs::write(
        &history_path,
        serialized,
    )
    .map_err(
        |error| {
            format!(
                "Unable to write recent shader history {}: {}",
                history_path.display(),
                error,
            )
        }
    )
}


fn promote_recent_shader_path(
    recent_paths: &mut Vec<PathBuf>,
    path: PathBuf,
) {
    recent_paths.retain(
        |existing| {
            existing != &path
        }
    );

    recent_paths.insert(
        0,
        path,
    );

    recent_paths.truncate(
        RECENT_SHADER_LIMIT
    );
}


fn is_supported_shader_path(
    path: &Path,
) -> bool {
    path.extension()
        .and_then(
            |extension| {
                extension.to_str()
            }
        )
        .is_some_and(
            |extension| {
                extension.eq_ignore_ascii_case(
                    "glsl"
                )
                    || extension.eq_ignore_ascii_case(
                        "fs"
                    )
            }
        )
}


fn parse_preview_selection(
    texture_specification: Option<&TextureSpecification>,
    palette_name: Option<&str>,
) -> Result<
    crate::manage_textures::PreviewTextureSelection,
    String,
> {

let texture =
    match texture_specification.cloned() {

        Some(specification) => {

            Some(
                crate::manage_textures::PreviewSelectionValue::Specific(
                    specification
                )
            )
        }

        None => {
            None
        }
    };


    let palette =
        match palette_name {

            Some(
                "random"
            ) => {
                Some(
                    crate::manage_textures::PreviewSelectionValue::Random
                )
            }

            Some(
                name
            ) => {
                Some(
                    crate::manage_textures::PreviewSelectionValue::Specific(
                        crate::palettes::Palette::from_name(
                            name
                        )?
                    )
                )
            }

            None => {
                None
            }
        };


    Ok(
        crate::manage_textures::PreviewTextureSelection {
            texture,
            palette,
        }
    )
}


fn discard_startup_input(
    event_pump: &mut sdl2::EventPump,
) {

    let input_arm_time =
        Instant::now()
            + Duration::from_millis(
                500
            );


    while Instant::now()
        < input_arm_time
    {
        for _event in
            event_pump.poll_iter()
        {
            // Intentionally discarded.
        }


        std::thread::sleep(
            Duration::from_millis(
                10
            )
        );
    }
}


fn save_control_configuration(
    control:
        &crate::editor_layout::ControlConfiguration,
) -> Result<crate::load_config::Config, String> {

    fn parse_global_texture(
        value: &str,
    ) -> Result<
        Option<
            crate::parse_texture_specification::TextureSpecification
        >,
        String,
    > {
        let normalized =
            value
                .trim()
                .to_ascii_lowercase();

        if normalized.is_empty()
            || normalized == "random"
        {
            return Ok(
                None
            );
        }

        crate::parse_texture_specification::parse_texture_specification(
            &normalized
        )
        .map(
            Some
        )
    }


    fn parse_global_palette(
        value: &str,
    ) -> Result<
        Option<
            crate::palettes::Palette
        >,
        String,
    > {
        let normalized =
            value
                .trim()
                .to_ascii_lowercase();

        if normalized.is_empty()
            || normalized == "random"
        {
            return Ok(
                None
            );
        }

        crate::palettes::Palette::from_name(
            &normalized
        )
        .map(
            Some
        )
    }


    fn build_mode(
        display: &str,
        interval_seconds: u64,
        single_filename: &str,
    ) -> Result<String, String> {

        match display
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "ordered" => {
                crate::manage_configuration::format_rotation_mode(
                    crate::manage_configuration::RotationMode::Ordered,
                    interval_seconds,
                )
            }

            "random" => {
                crate::manage_configuration::format_rotation_mode(
                    crate::manage_configuration::RotationMode::Random,
                    interval_seconds,
                )
            }
            "single" => {
                let filename =
                    single_filename
                        .trim();

                if filename.is_empty() {
                    Err(
                        "Single display mode requires a shader filename."
                            .to_string()
                    )
                } else {
                    Ok(
                        format!(
                            "single:{}",
                            filename,
                        )
                    )
                }
            }

            other => {
                Err(
                    format!(
                        "Unsupported display mode '{}'.",
                        other,
                    )
                )
            }
        }
    }


    let updates =
        crate::manage_configuration::ConfigurationUpdates {
            screensaver_enabled:
                control.screensaver_enabled,

            subtitles:
                control.subtitles,

            screensaver_mode:
                build_mode(
                    &control.screensaver_display,
                    control.screensaver_interval_seconds,
                    &control.screensaver_single_filename,
                )?,

            idle_timeout:
                control.idle_timeout.clone(),

            screensaver_global_texture:
                parse_global_texture(
                    &control.screensaver_global_texture
                )?,

            screensaver_global_palette:
                parse_global_palette(
                    &control.screensaver_global_palette
                )?,

            wallpaper_enabled:
                control.wallpaper_enabled,

            notifications:
                control.notifications,

            wallpaper_mode:
                build_mode(
                    &control.wallpaper_display,
                    control.wallpaper_interval_seconds,
                    &control.wallpaper_single_filename,
                )?,

            wallpaper_global_texture:
                parse_global_texture(
                    &control.wallpaper_global_texture
                )?,

            wallpaper_global_palette:
                parse_global_palette(
                    &control.wallpaper_global_palette
                )?,
        };


    let config_path =
        crate::locate_paths::config_path();


    crate::manage_configuration::save_configuration(
        &config_path,
        &updates,
    )?;


    crate::load_config::load_config(
        &config_path
    )
    .map(
        |result| {
            result.config
        }
    )
}


fn log_warning(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::warning(
        &logfile,
        message,
    );
}


fn log_information(
    message: &str,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        message,
    );
}



fn anti_aliasing_selection_from_method(
    method: crate::render_fxaa::AntiAliasingMethod,
) -> crate::editor_layout::AntiAliasingSelection {
    match method {
        crate::render_fxaa::AntiAliasingMethod::Off => {
            crate::editor_layout::AntiAliasingSelection::Off
        }

        crate::render_fxaa::AntiAliasingMethod::Fxaa => {
            crate::editor_layout::AntiAliasingSelection::Fxaa
        }
    }
}


fn anti_aliasing_method_from_selection(
    selection: crate::editor_layout::AntiAliasingSelection,
) -> crate::render_fxaa::AntiAliasingMethod {
    match selection {
        crate::editor_layout::AntiAliasingSelection::Off => {
            crate::render_fxaa::AntiAliasingMethod::Off
        }

        crate::editor_layout::AntiAliasingSelection::Fxaa => {
            crate::render_fxaa::AntiAliasingMethod::Fxaa
        }
    }
}


fn dithering_selection_from_level(
    level: crate::render_dithering::DitheringLevel,
) -> crate::editor_layout::DitheringSelection {
    match level {
        crate::render_dithering::DitheringLevel::Off => {
            crate::editor_layout::DitheringSelection::Off
        }

        crate::render_dithering::DitheringLevel::Subtle => {
            crate::editor_layout::DitheringSelection::Subtle
        }
    }
}


fn dithering_level_from_selection(
    selection: crate::editor_layout::DitheringSelection,
) -> crate::render_dithering::DitheringLevel {
    match selection {
        crate::editor_layout::DitheringSelection::Off => {
            crate::render_dithering::DitheringLevel::Off
        }

        crate::editor_layout::DitheringSelection::Subtle => {
            crate::render_dithering::DitheringLevel::Subtle
        }
    }
}


fn color_precision_selection_from_policy(
    policy: crate::select_render_precision::ColorPrecisionPolicy,
) -> crate::editor_layout::ColorPrecisionSelection {
    crate::editor_layout::ColorPrecisionSelection::from_policy(
        policy
    )
}


fn color_precision_policy_from_selection(
    selection: crate::editor_layout::ColorPrecisionSelection,
) -> crate::select_render_precision::ColorPrecisionPolicy {
    selection.policy()
}


unsafe fn set_uniform_1f(
    program: u32,
    name: &[u8],
    value: f32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform1f(
            location,
            value,
        );
    }
}


unsafe fn set_uniform_1i(
    program: u32,
    name: &[u8],
    value: i32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform1i(
            location,
            value,
        );
    }
}


unsafe fn set_uniform_3f(
    program: u32,
    name: &[u8],
    x: f32,
    y: f32,
    z: f32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform3f(
            location,
            x,
            y,
            z,
        );
    }
}


unsafe fn set_uniform_4f(
    program: u32,
    name: &[u8],
    x: f32,
    y: f32,
    z: f32,
    w: f32,
) {

    let location =
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                .cast(),
        );


    if location
        != -1
    {
        gl::Uniform4f(
            location,
            x,
            y,
            z,
            w,
        );
    }
}

