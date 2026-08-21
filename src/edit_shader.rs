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
use sdl2::keyboard::{
    Keycode,
    Scancode,
};
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


#[derive(
    serde::Serialize,
    serde::Deserialize,
    Default,
)]
struct ControlCenterState {

    #[serde(default)]
    recent_shaders:
        Vec<String>,

    #[serde(default)]
    policy_list:
        PersistentPolicyListState,

    #[serde(default)]
    window:
        PersistentWindowState,
}


#[derive(
    serde::Serialize,
    serde::Deserialize,
    Default,
)]
struct PersistentWindowState {

    #[serde(default)]
    x:
        Option<i32>,

    #[serde(default)]
    y:
        Option<i32>,
}


#[derive(
    serde::Serialize,
    serde::Deserialize,
)]
struct PersistentPolicyListState {

    #[serde(default = "default_policy_sort_column")]
    sort_column:
        String,

    #[serde(default = "default_true")]
    sort_ascending:
        bool,

    #[serde(default)]
    last_edited_policy:
        Option<PersistentPolicyIdentity>,
}


impl Default
    for PersistentPolicyListState
{
    fn default() -> Self {
        Self {
            sort_column:
                default_policy_sort_column(),

            sort_ascending:
                true,

            last_edited_policy:
                None,
        }
    }
}


#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
)]
struct PersistentPolicyIdentity {
    #[serde(default)]
    policy_id:
        Option<i64>,

    policy_key:
        String,

    policy_target:
        String,

    source_path:
        String,
}


fn protected_bulk_target_skip_count(
    patches: &[crate::manage_policies::BulkPolicyPatch],
    rows: &[crate::editor_layout::PolicyRowReference],
) -> usize {
    patches
        .iter()
        .zip(rows.iter())
        .filter(
            |(patch, row)| {
                patch.fields.policy_target
                    && patch.destination_target
                        .map(
                            |destination| {
                                destination != patch.current_target
                            }
                        )
                        .unwrap_or(false)
                    && crate::manage_policies::is_protected_default_policy(
                        row.policy_id
                    )
                    .unwrap_or(false)
            }
        )
        .count()
}


fn bulk_edit_completion_message(
    changed: usize,
    protected_target_skips: usize,
) -> String {
    if protected_target_skips == 0 {
        return format!(
            "Bulk Edit complete: {} policies updated.",
            changed,
        );
    }

    if changed == 0 {
        return format!(
            "No policies changed. Policy Target cannot be changed for {} protected default {}.",
            protected_target_skips,
            if protected_target_skips == 1 {
                "policy"
            } else {
                "policies"
            },
        );
    }

    format!(
        "Bulk Edit complete: {} policies updated. Policy Target was preserved for {} protected default {}.",
        changed,
        protected_target_skips,
        if protected_target_skips == 1 {
            "policy"
        } else {
            "policies"
        },
    )
}


fn process_policy_rename_ui(
    edit_window:
        &mut crate::editor_layout::EditWindowOverlay,
    editor_output:
        &crate::editor_layout::EditorOutput,
    config:
        &mut crate::load_config::Config,
    policy_display_rows:
        &mut Vec<crate::editor_layout::PolicyDisplayRow>,
) {
    if let Some((
        row,
        requested_name,
    )) =
        editor_output
            .rename_policy_requested
            .as_ref()
    {
        let manage_target =
            match row.policy_target {
                crate::editor_layout::PolicyTarget::Screensaver =>
                    crate::manage_policies::PolicyTarget::Screensaver,
                crate::editor_layout::PolicyTarget::Wallpaper =>
                    crate::manage_policies::PolicyTarget::Wallpaper,
                crate::editor_layout::PolicyTarget::Unassigned =>
                    crate::manage_policies::PolicyTarget::Unassigned,
            };

        match crate::manage_policies::rename_policy_by_id(
            row.policy_id,
            requested_name,
        ) {
            Ok(()) => {
                match crate::load_config::load_config(
                    &crate::locate_paths::config_path()
                ) {
                    Ok(reloaded_config) => {
                        *config =
                            reloaded_config.config;

                        *policy_display_rows =
                            build_policy_display_rows(
                                config
                            );

                        if let Some(
                            renamed_row
                        ) =
                            policy_display_rows
                                .iter()
                                .find(
                                    |candidate| {
                                        candidate.policy_id == row.policy_id
                                    }
                                )
                        {
                            edit_window.select_policy_row_persistently(
                                crate::editor_layout::PolicyRowReference {
                                    policy_id:
                                        renamed_row.policy_id,

                                    policy_key:
                                        renamed_row.policy_key.clone(),
                                    filename:
                                        renamed_row.filename.clone(),
                                    full_path:
                                        renamed_row.full_path.clone(),
                                    policy_target:
                                        renamed_row.policy_target,
                                    unassigned:
                                        renamed_row.unassigned,
                                }
                            );
                        }

                        edit_window.complete_policy_rename();

                        edit_window.set_status_message(
                            format!(
                                "Policy renamed to '{}'.",
                                requested_name,
                            )
                        );

                        log_information(
                            &format!(
                                "[EDIT_SHADER] Renamed policy '{}' as '{}'",
                                row.policy_key,
                                requested_name,
                            )
                        );
                    }

                    Err(error) => {
                        edit_window.set_policy_rename_validation_message(
                            format!(
                                "Policy was renamed, but configuration reload failed: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Policy '{}' renamed as '{}', but configuration reload failed: {}",
                                row.policy_key,
                                requested_name,
                                error,
                            )
                        );
                    }
                }
            }

            Err(error) => {
                edit_window.set_policy_rename_validation_message(
                    error.clone()
                );

                edit_window.set_status_message(
                    format!(
                        "Unable to rename policy: {}",
                        error,
                    )
                );

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Unable to rename policy '{}': {}",
                        row.policy_key,
                        error,
                    )
                );
            }
        }

        return;
    }

    if let Some((
        row,
        crate::editor_layout::PolicyRowCommand::RenamePolicy,
    )) =
        editor_output
            .policy_row_command_requested
            .as_ref()
    {
        edit_window.begin_policy_rename(
            row.clone()
        );
    }
}


fn process_policy_clone_ui(
    edit_window:
        &mut crate::editor_layout::EditWindowOverlay,
    editor_output:
        &crate::editor_layout::EditorOutput,
    config:
        &mut crate::load_config::Config,
    policy_display_rows:
        &mut Vec<crate::editor_layout::PolicyDisplayRow>,
) {
    if let Some((
        row,
        requested_name,
    )) =
        editor_output
            .clone_policy_requested
            .as_ref()
    {
        let manage_target =
            match row.policy_target {
                crate::editor_layout::PolicyTarget::Screensaver =>
                    crate::manage_policies::PolicyTarget::Screensaver,
                crate::editor_layout::PolicyTarget::Wallpaper =>
                    crate::manage_policies::PolicyTarget::Wallpaper,
                crate::editor_layout::PolicyTarget::Unassigned =>
                    crate::manage_policies::PolicyTarget::Unassigned,
            };

        match crate::manage_policies::clone_policy_by_id(
            row.policy_id,
            requested_name,
        ) {
            Ok(new_policy_id) => {
                match crate::load_config::load_config(
                    &crate::locate_paths::config_path()
                ) {
                    Ok(reloaded_config) => {
                        *config =
                            reloaded_config.config;

                        *policy_display_rows =
                            build_policy_display_rows(
                                config
                            );

                        if let Some(
                            clone_row
                        ) =
                            policy_display_rows
                                .iter()
                                .find(
                                    |candidate| {
                                        candidate.policy_id == new_policy_id
                                    }
                                )
                        {
                            edit_window.select_policy_row_persistently(
                                crate::editor_layout::PolicyRowReference {
                                    policy_id:
                                        clone_row.policy_id,

                                    policy_key:
                                        clone_row.policy_key.clone(),

                                    filename:
                                        clone_row.filename.clone(),

                                    full_path:
                                        clone_row.full_path.clone(),

                                    policy_target:
                                        clone_row.policy_target,

                                    unassigned:
                                        clone_row.unassigned,
                                }
                            );
                        }

                        edit_window.complete_policy_clone();

                        edit_window.set_status_message(
                            format!(
                                "Policy cloned as '{}'.",
                                requested_name,
                            )
                        );

                        log_information(
                            &format!(
                                "[EDIT_SHADER] Cloned policy '{}' as '{}'",
                                row.policy_key,
                                requested_name,
                            )
                        );
                    }

                    Err(error) => {
                        edit_window.set_policy_clone_validation_message(
                            format!(
                                "Policy was cloned, but configuration reload failed: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Policy '{}' cloned as '{}', but configuration reload failed: {}",
                                row.policy_key,
                                requested_name,
                                error,
                            )
                        );
                    }
                }
            }

            Err(error) => {
                edit_window.set_policy_clone_validation_message(
                    error.clone()
                );

                edit_window.set_status_message(
                    format!(
                        "Unable to clone policy: {}",
                        error,
                    )
                );

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Unable to clone policy '{}': {}",
                        row.policy_key,
                        error,
                    )
                );
            }
        }

        return;
    }


    if let Some((
        row,
        crate::editor_layout::PolicyRowCommand::ClonePolicy,
    )) =
        editor_output
            .policy_row_command_requested
            .as_ref()
    {
        let manage_target =
            match row.policy_target {
                crate::editor_layout::PolicyTarget::Screensaver =>
                    crate::manage_policies::PolicyTarget::Screensaver,
                crate::editor_layout::PolicyTarget::Wallpaper =>
                    crate::manage_policies::PolicyTarget::Wallpaper,
                crate::editor_layout::PolicyTarget::Unassigned =>
                    crate::manage_policies::PolicyTarget::Unassigned,
            };

        match crate::manage_policies::suggested_clone_policy_name(
            row.policy_id,
        ) {
            Ok(suggested_name) => {
                edit_window.begin_policy_clone(
                    row.clone(),
                    suggested_name,
                );
            }

            Err(error) => {
                edit_window.set_status_message(
                    format!(
                        "Unable to prepare policy clone: {}",
                        error,
                    )
                );

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Unable to prepare clone for policy '{}': {}",
                        row.policy_key,
                        error,
                    )
                );
            }
        }
    }
}


fn default_policy_sort_column() -> String {
    "filename".to_string()
}


fn default_true() -> bool {
    true
}


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
    audio_bands:
        Option<crate::audio_backend::SharedAudioBands>,
) -> Result<(), String> {

    let Some(shader_argument) =
        shader_argument
    else {
        return run_empty_session(
            audio_bands
        );
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
                None,
                None,
                audio_bands,
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
    audio_bands:
        Option<crate::audio_backend::SharedAudioBands>,
) -> Result<(), String> {
    // The system-tray Edit command opens the full Control Center, merely
    // seeding it with the currently active wallpaper and selecting the
    // Wallpaper policy target initially.  It must not remain restricted to
    // Wallpaper policies, because the user may load another shader and edit
    // either its Screensaver or Wallpaper policy from the same session.
    run_paths(
        vec![shader_path],
        None,
        None,
        None,
        None,
        None,
        EditorTargetRestriction::Unrestricted,
        Some(
            crate::editor_layout::PolicyTarget::Wallpaper
        ),
        None,
        None,
        audio_bands,
    )
}


pub fn run_screensaver_only(
    shader_path: PathBuf,
    audio_bands:
        Option<crate::audio_backend::SharedAudioBands>,
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
        None,
        None,
        audio_bands,
    )
}

fn run_empty_session(
    audio_bands:
        Option<crate::audio_backend::SharedAudioBands>,
) -> Result<(), String> {

    let wallpaper_pause_guard =
        crate::control_wallpaper::WallpaperPauseGuard::acquire();

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


    restore_policy_list_state(
        &mut edit_window,
        &policy_display_rows,
    );


    let mut last_saved_policy_list_state =
        edit_window
            .policy_list_state_snapshot();


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
            Option<i64>,
            Option<String>,
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
                edit_window.request_close();
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
                crate::editor_layout::BloomSelection::Off,
                crate::render_bloom::BLOOM_INTENSITY_DEFAULT,
                crate::render_bloom::BLOOM_THRESHOLD_DEFAULT,
                false,
                false,
                false,
                crate::postprocess_shader::HUE_ROTATION_DEFAULT,
                None,
                false,
                false,
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

        process_policy_rename_ui(
            &mut edit_window,
            &editor_output,
            &mut config,
            &mut policy_display_rows,
        );


        process_policy_clone_ui(
            &mut edit_window,
            &editor_output,
            &mut config,
            &mut policy_display_rows,
        );


        save_policy_list_state_if_changed(
            &edit_window,
            &mut last_saved_policy_list_state,
        );


        if editor_output.exit_discard_requested {
            break 'edit_session;
        }


        let mut exit_save_failed =
            false;


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
                        if editor_output.exit_after_save_requested {
                            exit_save_failed =
                                true;
                        }

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


        if editor_output.exit_after_save_requested
            && !exit_save_failed
            && !editor_output.bulk_save_requested
        {
            break 'edit_session;
        }


        if editor_output.bulk_save_requested {
            log_information(
                &format!(
                    "[EDIT_SHADER] Confirmed Bulk Edit save request received: selected_policies={}, changes={:?}",
                    editor_output.bulk_selected_policy_rows.len(),
                    editor_output.bulk_edit_changes,
                )
            );

            if !editor_output.bulk_edit_changes.any() {
                log_warning(
                    "[EDIT_SHADER] Bulk Edit save request contained an empty field-change mask; no database update was attempted"
                );
                edit_window.set_status_message(
                    "Bulk Edit contains no changed settings."
                );
                continue;
            }

            let mut patches =
                Vec::with_capacity(
                    editor_output
                        .bulk_selected_policy_rows
                        .len()
                );

            let mut preparation_error:
                Option<String> =
                None;

            for row in
                &editor_output.bulk_selected_policy_rows
            {
                let shader_path =
                    PathBuf::from(
                        &row.full_path
                    );

                let texture_fields_changed =
                    editor_output.bulk_edit_changes.texture
                        || editor_output.bulk_edit_changes.palette
                        || editor_output.bulk_edit_changes.primitive_count;

                let texture_required =
                    if texture_fields_changed {
                        match shader_requires_texture_for_bulk_edit(
                            &shader_path
                        ) {
                            Ok(required) => required,
                            Err(error) => {
                                preparation_error =
                                    Some(error);
                                break;
                            }
                        }
                    } else {
                        false
                    };

                patches.push(
                    bulk_policy_patch_from_editor_output(
                        row,
                        &editor_output,
                        texture_required,
                    )
                );
            }

            if let Some(error) =
                preparation_error
            {
                edit_window.set_status_message(
                    format!(
                        "Bulk policy save aborted: {}",
                        error,
                    )
                );

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Bulk policy save aborted before database transaction: {}",
                        error,
                    )
                );

                continue;
            }

            let protected_target_skips =
                protected_bulk_target_skip_count(
                    &patches,
                    &editor_output.bulk_selected_policy_rows,
                );

            match crate::manage_policies::patch_policies_by_id(
                &patches
            ) {
                Ok(changed) => {
                    let config_path =
                        crate::locate_paths::config_path();

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

                            edit_window.complete_bulk_save(
                                false
                            );

                            edit_window.set_status_message(
                                bulk_edit_completion_message(
                                    changed,
                                    protected_target_skips,
                                )
                            );

                            log_information(
                                &format!(
                                    "[EDIT_SHADER] Bulk Edit updated {} policies in one database transaction",
                                    changed,
                                )
                            );

                            if editor_output.exit_after_save_requested {
                                break 'edit_session;
                            }
                        }

                        Err(error) => {
                            edit_window.set_status_message(
                                "Bulk policies were saved, but configuration reload failed."
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Bulk policies were saved, but configuration reload failed: {}",
                                    error,
                                )
                            );
                        }
                    }
                }

                Err(error) => {
                    edit_window.set_status_message(
                        format!(
                            "Unable to save bulk policy changes: {}",
                            error,
                        )
                    );

                    log_warning(
                        &format!(
                            "[EDIT_SHADER] Bulk policy save failed: {}",
                            error,
                        )
                    );
                }
            }

            continue;
        }


        if editor_output.bulk_create_browse_requested {
            let starting_directory =
                crate::locate_paths::shader_dir();


            let selected_paths =
                rfd::FileDialog::new()
                    .set_parent(
                        &window
                    )
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
                    .pick_files();


            if let Err(error) =
                restore_editor_fullscreen(
                    &mut window
                )
            {
                log_warning(
                    &format!(
                        "[EDIT_SHADER] Immediate fullscreen restoration failed after bulk file selection: {}",
                        error,
                    )
                );
            }


            let Some(selected_paths) =
                selected_paths
            else {
                edit_window.set_status_message(
                    "Bulk policy creation canceled."
                );

                continue;
            };


            let (
                candidates,
                rejected_count,
            ) =
                analyze_bulk_policy_candidates(
                    selected_paths
                );


            if candidates.is_empty() {
                edit_window.set_status_message(
                    "No usable shaders were selected for policy creation."
                );

                continue;
            }


            edit_window.begin_bulk_policy_creation(
                candidates,
                rejected_count,
            );

            continue;
        }


        if let Some(request) =
            editor_output.bulk_create_requested
                .as_ref()
        {
            match create_bulk_policies(
                request,
                &editor_output,
            ) {
                Ok(result) => {
                    match crate::load_config::load_config(
                        &crate::locate_paths::config_path()
                    ) {
                        Ok(reloaded_config) => {
                            config =
                                reloaded_config.config;

                            policy_display_rows =
                                build_policy_display_rows(
                                    &config
                                );

                            edit_window.complete_bulk_policy_creation();

                            edit_window.set_status_message(
                                format!(
                                    "Bulk policy creation complete: {} created, {} already existed.",
                                    result.created,
                                    result.skipped_existing,
                                )
                            );

                            log_information(
                                &format!(
                                    "[EDIT_SHADER] Bulk policy creation completed: {} created, {} existing policies skipped",
                                    result.created,
                                    result.skipped_existing,
                                )
                            );
                        }

                        Err(error) => {
                            edit_window.set_status_message(
                                "Policies were created, but configuration reload failed."
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Bulk policies created, but configuration reload failed: {}",
                                    error,
                                )
                            );
                        }
                    }
                }

                Err(error) => {
                    edit_window.begin_bulk_policy_creation(
                        request.candidates.clone(),
                        request.rejected_count,
                    );

                    edit_window.set_status_message(
                        format!(
                            "Bulk policy creation failed: {}",
                            error,
                        )
                    );

                    log_warning(
                        &format!(
                            "[EDIT_SHADER] Bulk policy creation failed: {}",
                            error,
                        )
                    );
                }
            }

            continue;
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
                        crate::locate_paths::shader_dir();

                    let selected_path =
                        rfd::FileDialog::new()
                    .set_parent(
                        &window
                    )
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
                        None,
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
        {
            if let Some(destination_target) =
                policy_move_destination(
                    *command
                )
            {
                match move_policy_shader(
                    &config,
                    row,
                    destination_target,
                ) {
                    Ok(destination_path) => {
                        match crate::load_config::load_config(
                            &crate::locate_paths::config_path()
                        ) {
                            Ok(reloaded_config) => {
                                config =
                                    reloaded_config.config;

                                policy_display_rows =
                                    build_policy_display_rows(
                                        &config
                                    );

                                edit_window.set_status_message(
                                    format!(
                                        "Shader moved to {}.",
                                        destination_path
                                            .parent()
                                            .map(
                                                |path| path.display().to_string()
                                            )
                                            .unwrap_or_default(),
                                    )
                                );

                                log_information(
                                    &format!(
                                        "[EDIT_SHADER] Moved shader {} to {}",
                                        row.filename,
                                        destination_path.display(),
                                    )
                                );
                            }

                            Err(error) => {
                                edit_window.set_status_message(
                                    "Shader moved; configuration reload failed."
                                );

                                log_warning(
                                    &format!(
                                        "[EDIT_SHADER] Shader moved, but configuration reload failed: {}",
                                        error,
                                    )
                                );
                            }
                        }
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            error.clone()
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to move shader {}: {}",
                                row.filename,
                                error,
                            )
                        );
                    }
                }

                continue;
            }
        }


        if let Some((
            row,
            crate::editor_layout::PolicyRowCommand::DeletePolicy,
        )) =
            editor_output
                .policy_row_command_requested
                .as_ref()
        {
            let manage_target =
                match row.policy_target {
                    crate::editor_layout::PolicyTarget::Screensaver => {
                        crate::manage_policies::PolicyTarget::Screensaver
                    }

                    crate::editor_layout::PolicyTarget::Wallpaper => {
                        crate::manage_policies::PolicyTarget::Wallpaper
                    }

                    crate::editor_layout::PolicyTarget::Unassigned => {
                        crate::manage_policies::PolicyTarget::Unassigned
                    }
                };


            let config_path =
                crate::locate_paths::config_path();


            match crate::manage_policies::delete_policy_by_id(
                &config_path,
                row.policy_id,
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
                                    "[EDIT_SHADER] Policy deleted from empty Control Center session, but configuration reload failed: {}",
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
                            "[EDIT_SHADER] Deleted {} policy for {} from empty Control Center session",
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
                            "[EDIT_SHADER] Unable to delete {} policy for {} from empty Control Center session: {}",
                            manage_target.name(),
                            row.filename,
                            error,
                        )
                    );
                }
            }


            continue;
        }


        if let Some((
            row,
            crate::editor_layout::PolicyRowCommand::DeleteShader,
        )) =
            editor_output
                .policy_row_command_requested
                .as_ref()
        {
            let manage_target =
                match row.policy_target {
                    crate::editor_layout::PolicyTarget::Screensaver => {
                        crate::manage_policies::PolicyTarget::Screensaver
                    }

                    crate::editor_layout::PolicyTarget::Wallpaper => {
                        crate::manage_policies::PolicyTarget::Wallpaper
                    }

                    crate::editor_layout::PolicyTarget::Unassigned => {
                        crate::manage_policies::PolicyTarget::Unassigned
                    }
                };


            let shader_path =
                PathBuf::from(
                    &row.full_path
                );


            let config_path =
                crate::locate_paths::config_path();


            if !shader_path.is_file() {
                edit_window.set_status_message(
                    format!(
                        "Shader file is unavailable: {}",
                        shader_path.display(),
                    )
                );


                log_warning(
                    &format!(
                        "[EDIT_SHADER] Refusing to delete unavailable shader {} from empty Control Center session",
                        shader_path.display(),
                    )
                );


                continue;
            }


            match crate::manage_policies::delete_policy_by_id(
                &config_path,
                row.policy_id,
            ) {
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
                                            "[EDIT_SHADER] Shader/policy deleted from empty Control Center session, but configuration reload failed: {}",
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
                                    "[EDIT_SHADER] Deleted {} shader {} and its policy from empty Control Center session",
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
                                    "[EDIT_SHADER] Deleted {} policy for {}, but failed to delete shader file {} from empty Control Center session: {}",
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
                            "[EDIT_SHADER] Refusing to delete shader {} from empty Control Center session because {} policy deletion failed: {}",
                            shader_path.display(),
                            manage_target.name(),
                            error,
                        )
                    );
                }
            }


            continue;
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
                PathBuf::from(
                    &row.full_path
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
                        Some(
                            row.policy_id
                        ),
                        Some(
                            row.policy_key.clone()
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
                        crate::locate_paths::shader_dir()
                    }

                    crate::editor_layout::PolicyTarget::Wallpaper => {
                        crate::locate_paths::shader_dir()
                    }

                    crate::editor_layout::PolicyTarget::Unassigned => {
                        crate::locate_paths::shader_dir()
                    }
                };

            let selected_path =
                rfd::FileDialog::new()
                    .set_parent(
                        &window
                    )
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
        policy_id,
        policy_name,
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
            policy_id,
            policy_name,
            audio_bands,
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
    requested_initial_policy_id: Option<i64>,
    requested_initial_policy_name: Option<String>,
    audio_bands:
        Option<crate::audio_backend::SharedAudioBands>,
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


    let initial_shader_path =
        shader_paths
            .first()
            .expect(
                "shader_paths was checked for emptiness"
            );


    let initial_managed_target =
        managed_policy_target_for_path(
            initial_shader_path
        );


    let (
        mut screensaver_target_available,
        mut wallpaper_target_available,
    ) =
        match target_restriction {
            EditorTargetRestriction::WallpaperOnly => {
                (
                    false,
                    true,
                )
            }

            EditorTargetRestriction::ScreensaverOnly => {
                (
                    true,
                    false,
                )
            }

            EditorTargetRestriction::Unrestricted => {
                match initial_managed_target {
                    Some(
                        crate::editor_layout::PolicyTarget::Screensaver
                    ) => {
                        (
                            true,
                            false,
                        )
                    }

                    Some(
                        crate::editor_layout::PolicyTarget::Wallpaper
                    ) => {
                        (
                            false,
                            true,
                        )
                    }

                    Some(
                        crate::editor_layout::PolicyTarget::Unassigned
                    ) => {
                        (
                            true,
                            true,
                        )
                    }

                    None => {
                        // Shaders outside Screenshaver's managed folders remain
                        // intentionally unrestricted and may be assigned to
                        // either runtime target.
                        (
                            true,
                            true,
                        )
                    }
                }
            }
        };


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
        if target_restriction
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
        } else if let Some(
            managed_target
        ) =
            initial_managed_target
        {
            Some(
                managed_target
            )
        } else if let Some(
            requested_initial_target
        ) =
            requested_initial_target
        {
            Some(
                requested_initial_target
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
    ) =
        editor_policy_context_for_path(
            &config,
            initial_editor_target,
            initial_shader_path,
            requested_initial_policy_id,
            requested_initial_policy_name.as_deref(),
            command_line_animation_speed,
        );


    let initial_policy_exists =
        match initial_editor_target {
            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ) => {
                screensaver_policy_exists
            }

            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ) => {
                wallpaper_policy_exists
            }

            Some(
                crate::editor_layout::PolicyTarget::Unassigned
            ) => {
                config.unassigned_policies
                    .iter()
                    .any(
                        |policy| {
                            policy_applies_to_path(
                                policy,
                                crate::editor_layout::PolicyTarget::Unassigned,
                                initial_shader_path,
                            )
                        }
                    )
            }

            None => {
                false
            }
        };


    let startup_status =
        match initial_editor_target {

            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ) if initial_policy_exists => {
                "Loaded existing Wallpaper policy for this shader."
                    .to_string()
            }

            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ) => {
                "Wallpaper target enforced by shader location. New Wallpaper policy is ready to save."
                    .to_string()
            }

            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ) if initial_policy_exists => {
                "Loaded existing Screensaver policy for this shader."
                    .to_string()
            }

            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ) => {
                "Screensaver target enforced by shader location. New Screensaver policy is ready to save."
                    .to_string()
            }

            Some(
                crate::editor_layout::PolicyTarget::Unassigned
            ) if initial_policy_exists => {
                "Loaded existing Unassigned policy for this shader."
                    .to_string()
            }

            Some(
                crate::editor_layout::PolicyTarget::Unassigned
            ) => {
                "New Unassigned policy is ready to save."
                    .to_string()
            }

            None => {
                "No existing shader policy found. Select a policy target to create one."
                    .to_string()
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


    restore_policy_list_state(
        &mut edit_window,
        &policy_display_rows,
    );


    let mut last_saved_policy_list_state =
        edit_window
            .policy_list_state_snapshot();


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
            &active.shader_name,
            Some(
                active.path.as_path()
            ),
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
        bloom_selection_from_mode(
            live_postprocess_profile.bloom
        ),
        live_postprocess_profile.bloom_intensity,
        live_postprocess_profile.bloom_threshold,
        live_postprocess_profile.invert_colors,
        live_postprocess_profile.flip_horizontal,
        live_postprocess_profile.flip_vertical,
        live_postprocess_profile.hue_rotation,
        active.texture_manager
            .active_specification_selection(),
        initial_policy_exists,
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


    // Bulk Edit temporarily borrows the Control Center from the currently
    // loaded shader.  The GL resources are destroyed on entry, while the
    // editor widget state (including unsaved single-policy edits) remains
    // untouched inside EditWindowOverlay.
    let mut bulk_edit_preview_suspended =
        false;

    let mut suspended_shader_id:
        Option<i64> =
        None;

    let mut suspended_preferred_target =
        initial_editor_target;

    let mut last_non_bulk_policy_target =
        initial_editor_target;


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
                        edit_window.request_close();
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


            if bulk_edit_preview_suspended {
                let (
                    suspended_width,
                    suspended_height,
                ) =
                    window.drawable_size();


                unsafe {
                    gl::Viewport(
                        0,
                        0,
                        suspended_width.min(i32::MAX as u32) as i32,
                        suspended_height.min(i32::MAX as u32) as i32,
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
                        bloom_selection_from_mode(
                            live_postprocess_profile
                                .bloom
                        ),
                        live_postprocess_profile
                            .bloom_intensity,
                        live_postprocess_profile
                            .bloom_threshold,
                        live_postprocess_profile
                            .invert_colors,
                        live_postprocess_profile
                            .flip_horizontal,
                        live_postprocess_profile
                            .flip_vertical,
                        live_postprocess_profile
                            .hue_rotation,
                        None,
                        true,
                        false,
                        screensaver_policy_exists,
                        wallpaper_policy_exists,
                        true,
                        true,
                        false,
                        false,
                        &recent_shader_paths,
                        None,
                        &policy_display_rows,
                        Some(&config),
                    );


                // The active shader has intentionally been unloaded during
                // Bulk Edit.  Present the cleared black frame plus the egui
                // Control Center overlay every iteration; otherwise the
                // front buffer retains the final shader frame and appears
                // to have "frozen".
                window.gl_swap_window();


                process_policy_rename_ui(
                    &mut edit_window,
                    &editor_output,
                    &mut config,
                    &mut policy_display_rows,
                );


                process_policy_clone_ui(
                    &mut edit_window,
                    &editor_output,
                    &mut config,
                    &mut policy_display_rows,
                );


            save_policy_list_state_if_changed(
                    &edit_window,
                    &mut last_saved_policy_list_state,
                );


                if editor_output.exit_discard_requested
                    || !editor_output.window_open
                {
                    break 'preview Ok(());
                }


                let mut restore_after_bulk =
                    editor_output
                        .bulk_selected_policy_rows
                        .len()
                        < 2
                        || editor_output.cancel_requested;


                if editor_output.bulk_save_requested {
                    log_information(
                        &format!(
                            "[EDIT_SHADER] Confirmed Bulk Edit save request received while preview was suspended: selected_policies={}, changes={:?}",
                            editor_output.bulk_selected_policy_rows.len(),
                            editor_output.bulk_edit_changes,
                        )
                    );

                    if !editor_output.bulk_edit_changes.any() {
                        log_warning(
                            "[EDIT_SHADER] Bulk Edit save request contained an empty field-change mask while preview was suspended; no database update was attempted"
                        );
                        edit_window.set_status_message(
                            "Bulk Edit contains no changed settings."
                        );
                    } else {
                        let mut patches =
                            Vec::with_capacity(
                                editor_output
                                    .bulk_selected_policy_rows
                                    .len()
                            );

                        let mut preparation_error:
                            Option<String> =
                            None;

                        for row in
                            &editor_output.bulk_selected_policy_rows
                        {
                            let shader_path =
                                PathBuf::from(
                                    &row.full_path
                                );

                            let texture_fields_changed =
                                editor_output.bulk_edit_changes.texture
                                    || editor_output.bulk_edit_changes.palette
                                    || editor_output.bulk_edit_changes.primitive_count;

                            let texture_required =
                                if texture_fields_changed {
                                    match shader_requires_texture_for_bulk_edit(
                                        &shader_path
                                    ) {
                                        Ok(required) => required,
                                        Err(error) => {
                                            preparation_error =
                                                Some(error);
                                            break;
                                        }
                                    }
                                } else {
                                    false
                                };

                            patches.push(
                                bulk_policy_patch_from_editor_output(
                                    row,
                                    &editor_output,
                                    texture_required,
                                )
                            );
                        }

                        if let Some(error) =
                            preparation_error
                        {
                            edit_window.set_status_message(
                                format!(
                                    "Bulk policy save aborted: {}",
                                    error,
                                )
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Bulk policy save aborted before database transaction: {}",
                                    error,
                                )
                            );
                        } else {
                            let protected_target_skips =
                                protected_bulk_target_skip_count(
                                    &patches,
                                    &editor_output.bulk_selected_policy_rows,
                                );

                            match crate::manage_policies::patch_policies_by_id(
                                &patches
                            ) {
                                Ok(changed) => {
                                    match crate::load_config::load_config(
                                        &crate::locate_paths::config_path()
                                    ) {
                                        Ok(reloaded_config) => {
                                            config =
                                                reloaded_config.config;

                                            policy_display_rows =
                                                build_policy_display_rows(
                                                    &config
                                                );

                                            edit_window.complete_bulk_save(
                                                false
                                            );

                                            edit_window.set_status_message(
                                                bulk_edit_completion_message(
                                                    changed,
                                                    protected_target_skips,
                                                )
                                            );

                                            log_information(
                                                &format!(
                                                    "[EDIT_SHADER] Bulk Edit updated {} policies while preview was suspended",
                                                    changed,
                                                )
                                            );

                                            restore_after_bulk =
                                                true;

                                            if editor_output.exit_after_save_requested {
                                                break 'preview Ok(());
                                            }
                                        }

                                        Err(error) => {
                                            edit_window.set_status_message(
                                                "Bulk policies were saved, but configuration reload failed."
                                            );

                                            log_warning(
                                                &format!(
                                                    "[EDIT_SHADER] Bulk policies were saved, but configuration reload failed: {}",
                                                    error,
                                                )
                                            );
                                        }
                                    }
                                }

                                Err(error) => {
                                    edit_window.set_status_message(
                                        format!(
                                            "Unable to save bulk policy changes: {}",
                                            error,
                                        )
                                    );

                                    log_warning(
                                        &format!(
                                            "[EDIT_SHADER] Bulk policy save failed: {}",
                                            error,
                                        )
                                    );
                                }
                            }
                        }
                    }
                }


                if restore_after_bulk {
                    let Some(shader_id) =
                        suspended_shader_id
                    else {
                        bulk_edit_preview_suspended =
                            false;

                        continue;
                    };


                    match resolve_shader_by_id_for_control_center(
                        shader_id,
                        suspended_preferred_target,
                    ) {
                        Ok(
                            Some(
                                (
                                    restored_path,
                                    restored_target,
                                )
                            )
                        ) => {
                            let (
                                restored_global_fps,
                                restored_fps_entries,
                                restored_texture_policy,
                                restored_postprocess_policy,
                                restored_animation_speed,
                            ) =
                                editor_policy_context_for_path(
                                    &config,
                                    restored_target,
                                    &restored_path,
                                    edit_window
                                        .policy_list_state_snapshot()
                                        .selected_policy_row
                                        .as_ref()
                                        .map(
                                            |row| row.policy_id
                                        ),
                                    edit_window
                                        .policy_list_state_snapshot()
                                        .selected_policy_row
                                        .as_ref()
                                        .map(
                                            |row| row.policy_key.as_str()
                                        ),
                                    command_line_animation_speed,
                                );


                            global_rendered_fps =
                                restored_global_fps;

                            fps_policy_entries =
                                restored_fps_entries;

                            texture_policy =
                                restored_texture_policy;

                            postprocess_policy =
                                restored_postprocess_policy;

                            animation_speed =
                                restored_animation_speed;


                            configured_fps =
                                resolve_preview_fps(
                                    global_rendered_fps,
                                    &fps_policy_entries,
                                    command_line_fps,
                                    restored_path
                                        .file_name()
                                        .and_then(
                                            |name| name.to_str()
                                        )
                                        .unwrap_or_default(),
                                );


                            let restored_preview_selection =
                                parse_preview_selection(
                                    None,
                                    None,
                                )?;


                            match load_active_shader(
                                &restored_path,
                                &texture_policy,
                                restored_preview_selection,
                                subtitles,
                                subtitle_placement,
                                configured_fps,
                                animation_speed,
                                window.size().0,
                                window.size().1,
                            ) {
                                Ok(replacement) => {
                                    active =
                                        replacement;

                                    information_path =
                                        resolve_information_path(
                                            &active.path,
                                            &active.shader_name,
                                            restored_target,
                                        );


                                    screensaver_policy_exists =
                                        config.screensaver_policies
                                            .iter()
                                            .any(
                                                |policy| {
                                                    policy_applies_to_path(
                                                        policy,
                                                        crate::editor_layout::PolicyTarget::Screensaver,
                                                        &active.path,
                                                    )
                                                }
                                            );

                                    wallpaper_policy_exists =
                                        config.wallpaper_policies
                                            .iter()
                                            .any(
                                                |policy| {
                                                    policy_applies_to_path(
                                                        policy,
                                                        crate::editor_layout::PolicyTarget::Wallpaper,
                                                        &active.path,
                                                    )
                                                }
                                            );


                                    live_postprocess_profile =
                                        postprocess_policy
                                            .profile_for_shader(
                                                &active.shader_name,
                                                Some(
                                                    active.path.as_path()
                                                ),
                                            );

                                    render_scale =
                                        live_postprocess_profile
                                            .render_scale;

                                    postprocess.set_profile(
                                        live_postprocess_profile
                                    )?;


                                    target_frame_time =
                                        Duration::from_secs_f64(
                                            1.0
                                                / configured_fps.max(1) as f64
                                        );


                                    last_non_bulk_policy_target =
                                        restored_target;

                                    bulk_edit_preview_suspended =
                                        false;

                                    suspended_shader_id =
                                        None;

                                    suspended_preferred_target =
                                        restored_target;

                                    log_information(
                                        &format!(
                                            "[EDIT_SHADER] Restored shader {} after Bulk Edit using shader_id={}",
                                            active.path.display(),
                                            shader_id,
                                        )
                                    );
                                }

                                Err(error) => {
                                    bulk_edit_preview_suspended =
                                        false;

                                    suspended_shader_id =
                                        None;

                                    edit_window.set_status_message(
                                        format!(
                                            "Bulk Edit ended, but the previous shader could not be reloaded: {}",
                                            error,
                                        )
                                    );

                                    log_warning(
                                        &format!(
                                            "[EDIT_SHADER] Unable to restore shader_id={} after Bulk Edit: {}",
                                            shader_id,
                                            error,
                                        )
                                    );

                                    continue;
                                }
                            }
                        }

                        Ok(None) => {
                            bulk_edit_preview_suspended =
                                false;

                            suspended_shader_id =
                                None;

                            edit_window.set_status_message(
                                "Bulk Edit ended; the previously loaded shader is no longer available."
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] shader_id={} no longer resolves after Bulk Edit",
                                    shader_id,
                                )
                            );

                            continue;
                        }

                        Err(error) => {
                            edit_window.set_status_message(
                                format!(
                                    "Unable to restore the previous shader after Bulk Edit: {}",
                                    error,
                                )
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Unable to resolve shader_id={} after Bulk Edit: {}",
                                    shader_id,
                                    error,
                                )
                            );

                            continue;
                        }
                    }
                }


                continue;
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
                                    &active.shader_name,
                                    Some(
                                        active.path.as_path()
                                    ),
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


            // Audio Bloom consumes the latest backend-independent analyzer
            // output. If audio is unavailable (or the shared state cannot be
            // read), all three bands remain zero and Bloom contributes nothing.
            let current_audio_bands =
                audio_bands
                    .as_ref()
                    .and_then(
                        |shared| {
                            shared
                                .read()
                                .ok()
                                .map(
                                    |bands| *bands
                                )
                        }
                    )
                    .unwrap_or_default();

            postprocess.set_audio_bands(
                current_audio_bands
            );


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
            }


            if let Err(error) =
                active.texture_manager
                    .update_animations()
            {
                log_warning(
                    &format!(
                        "[TEXTURE] Unable to update animated preview texture: {error}"
                    )
                );
            }


            unsafe {
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


            let bloom_diagnostic =
                event_pump
                    .keyboard_state()
                    .is_scancode_pressed(
                        Scancode::LCtrl
                    )
                || event_pump
                    .keyboard_state()
                    .is_scancode_pressed(
                        Scancode::RCtrl
                    );


            postprocess
                .present_scene_with_bloom_diagnostic(
                    bloom_diagnostic
                );


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


            let active_policy_name =
                edit_window
                    .active_selected_policy_row()
                    .map(
                        |row| {
                            row.policy_key
                        }
                    )
                    .unwrap_or_else(
                        || {
                            "—".to_string()
                        }
                    );


            let shader_information =
                crate::editor_layout::ShaderInformation {
                    policy_name:
                        active_policy_name,

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
                    bloom_selection_from_mode(
                        live_postprocess_profile
                            .bloom
                    ),
                    live_postprocess_profile
                        .bloom_intensity,
                    live_postprocess_profile
                        .bloom_threshold,
                    live_postprocess_profile
                        .invert_colors,
                    live_postprocess_profile
                        .flip_horizontal,
                    live_postprocess_profile
                        .flip_vertical,
                    live_postprocess_profile
                        .hue_rotation,
                    active_texture_selection,
                    true,
                    active.channel_usage
                        .uses_any_channel(),
                    screensaver_policy_exists,
                    wallpaper_policy_exists,
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

            process_policy_rename_ui(
                &mut edit_window,
                &editor_output,
                &mut config,
                &mut policy_display_rows,
            );


            process_policy_clone_ui(
                &mut edit_window,
                &editor_output,
                &mut config,
                &mut policy_display_rows,
            );


            save_policy_list_state_if_changed(
            &edit_window,
            &mut last_saved_policy_list_state,
        );


            if editor_output
                .bulk_selected_policy_rows
                .len()
                > 1
            {
                match shader_id_for_control_center_path(
                    &active.path
                ) {
                    Ok(Some(shader_id)) => {
                        suspended_shader_id =
                            Some(
                                shader_id
                            );

                        suspended_preferred_target =
                            last_non_bulk_policy_target;

                        destroy_active_shader(
                            &mut active
                        );

                        bulk_edit_preview_suspended =
                            true;

                        log_information(
                            &format!(
                                "[EDIT_SHADER] Suspended active preview for Bulk Edit: shader_id={}",
                                shader_id,
                            )
                        );

                        continue;
                    }

                    Ok(None) => {
                        edit_window.set_status_message(
                            "Bulk Edit could not suspend the active shader because its database ID could not be found."
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to locate shader_id for active preview {}",
                                active.path.display(),
                            )
                        );
                    }

                    Err(error) => {
                        edit_window.set_status_message(
                            format!(
                                "Bulk Edit could not suspend the active shader: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Unable to resolve active shader ID for Bulk Edit: {}",
                                error,
                            )
                        );
                    }
                }
            }


            if let Some(target) =
                editor_output.policy_target
            {
                last_non_bulk_policy_target =
                    Some(
                        target
                    );
            }


        if editor_output.exit_discard_requested {
                break 'preview Ok(());
            }


            let mut exit_save_failed =
                false;


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
                            if editor_output.exit_after_save_requested {
                                exit_save_failed =
                                    true;
                            }

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
                            crate::locate_paths::shader_dir()
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            crate::locate_paths::shader_dir()
                        }

                        crate::editor_layout::PolicyTarget::Unassigned => {
                            crate::locate_paths::shader_dir()
                        }
                    };

                let selected_path =
                    rfd::FileDialog::new()
                    .set_parent(
                        &window
                    )
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


            if editor_output.bulk_create_browse_requested {
                let starting_directory =
                    active.path
                        .parent()
                        .unwrap_or_else(
                            || Path::new(".")
                        );


                let selected_paths =
                    rfd::FileDialog::new()
                    .set_parent(
                        &window
                    )
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
                        .pick_files();


                if let Err(error) =
                    restore_editor_fullscreen(
                        &mut window
                    )
                {
                    log_warning(
                        &format!(
                            "[EDIT_SHADER] Immediate fullscreen restoration failed after bulk file selection: {}",
                            error,
                        )
                    );
                }


                fullscreen_restore_requested_at =
                    Some(
                        Instant::now()
                    );


                let Some(selected_paths) =
                    selected_paths
                else {
                    edit_window.set_status_message(
                        "Bulk policy creation canceled."
                    );

                    continue;
                };


                let (
                    candidates,
                    rejected_count,
                ) =
                    analyze_bulk_policy_candidates(
                        selected_paths
                    );


                if candidates.is_empty() {
                    edit_window.set_status_message(
                        "No usable shaders were selected for policy creation."
                    );

                    continue;
                }


                edit_window.begin_bulk_policy_creation(
                    candidates,
                    rejected_count,
                );

                continue;
            }


            if let Some(request) =
                editor_output.bulk_create_requested
                    .as_ref()
            {
                match create_bulk_policies(
                    request,
                    &editor_output,
                ) {
                    Ok(result) => {
                        match crate::load_config::load_config(
                            &crate::locate_paths::config_path()
                        ) {
                            Ok(reloaded_config) => {
                                config =
                                    reloaded_config.config;

                                policy_display_rows =
                                    build_policy_display_rows(
                                        &config
                                    );

                                edit_window.complete_bulk_policy_creation();

                                edit_window.set_status_message(
                                    format!(
                                        "Bulk policy creation complete: {} created, {} already existed.",
                                        result.created,
                                        result.skipped_existing,
                                    )
                                );

                                log_information(
                                    &format!(
                                        "[EDIT_SHADER] Bulk policy creation completed: {} created, {} existing policies skipped",
                                        result.created,
                                        result.skipped_existing,
                                    )
                                );
                            }

                            Err(error) => {
                                edit_window.set_status_message(
                                    "Policies were created, but configuration reload failed."
                                );

                                log_warning(
                                    &format!(
                                        "[EDIT_SHADER] Bulk policies created, but configuration reload failed: {}",
                                        error,
                                    )
                                );
                            }
                        }
                    }

                    Err(error) => {
                        edit_window.begin_bulk_policy_creation(
                            request.candidates.clone(),
                            request.rejected_count,
                        );

                        edit_window.set_status_message(
                            format!(
                                "Bulk policy creation failed: {}",
                                error,
                            )
                        );

                        log_warning(
                            &format!(
                                "[EDIT_SHADER] Bulk policy creation failed: {}",
                                error,
                            )
                        );
                    }
                }

                continue;
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
                            PathBuf::from(
                                &row.full_path
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
                    .set_parent(
                        &window
                    )
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

                let new_managed_target =
                    managed_policy_target_for_path(
                        &selected_path
                    );


                let (
                    new_screensaver_target_available,
                    new_wallpaper_target_available,
                ) =
                    match target_restriction {
                        EditorTargetRestriction::WallpaperOnly => {
                            (
                                false,
                                true,
                            )
                        }

                        EditorTargetRestriction::ScreensaverOnly => {
                            (
                                true,
                                false,
                            )
                        }

                        EditorTargetRestriction::Unrestricted => {
                            match new_managed_target {
                                Some(
                                    crate::editor_layout::PolicyTarget::Screensaver
                                ) => {
                                    (
                                        true,
                                        false,
                                    )
                                }

                                Some(
                                    crate::editor_layout::PolicyTarget::Wallpaper
                                ) => {
                                    (
                                        false,
                                        true,
                                    )
                                }

                                Some(
                                    crate::editor_layout::PolicyTarget::Unassigned
                                ) => {
                                    (
                                        true,
                                        true,
                                    )
                                }

                                None => {
                                    (
                                        true,
                                        true,
                                    )
                                }
                            }
                        }
                    };

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
                    if target_restriction
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
                    } else if let Some(
                        managed_target
                    ) =
                        new_managed_target
                    {
                        Some(
                            managed_target
                        )
                    } else if let Some(
                        row_forced_target
                    ) = row_forced_target
                    {
                        Some(
                            row_forced_target
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
                ) =
                    editor_policy_context_for_path(
                        &config,
                        new_editor_target,
                        &selected_path,
                        policy_row_open_request
                            .as_ref()
                            .map(
                                |(
                                    row,
                                    _command,
                                )| {
                                    row.policy_id
                                }
                            ),
                        policy_row_open_request
                            .as_ref()
                            .map(
                                |(
                                    row,
                                    _command,
                                )| {
                                    row.policy_key.as_str()
                                }
                            ),
                        command_line_animation_speed,
                    );


                let new_policy_exists =
                    match new_editor_target {
                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        ) => {
                            new_screensaver_policy_exists
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        ) => {
                            new_wallpaper_policy_exists
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Unassigned
                        ) => {
                            config.unassigned_policies
                                .iter()
                                .any(
                                    |policy| {
                                        policy_applies_to_path(
                                            policy,
                                            crate::editor_layout::PolicyTarget::Unassigned,
                                            &selected_path,
                                        )
                                    }
                                )
                        }

                        None => {
                            false
                        }
                    };


                let load_status =
                    match new_editor_target {

                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        ) if new_policy_exists => {
                            "Loaded shader with its existing Wallpaper policy."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        ) if new_managed_target.is_some() => {
                            "Wallpaper target enforced by shader location. New Wallpaper policy is ready to save."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Wallpaper
                        ) => {
                            "No Wallpaper policy exists. Loaded Wallpaper defaults."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        ) if new_policy_exists => {
                            "Loaded shader with its existing Screensaver policy."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        ) if new_managed_target.is_some() => {
                            "Screensaver target enforced by shader location. New Screensaver policy is ready to save."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Screensaver
                        ) => {
                            "No Screensaver policy exists. Loaded Screensaver defaults."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Unassigned
                        ) if new_policy_exists => {
                            "Loaded shader with its existing Unassigned policy."
                                .to_string()
                        }

                        Some(
                            crate::editor_layout::PolicyTarget::Unassigned
                        ) => {
                            "No Unassigned policy exists. Loaded defaults for a new Unassigned policy."
                                .to_string()
                        }

                        None => {
                            "Loaded shader using resolved defaults. Select a policy target to create a policy."
                                .to_string()
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
                                    &replacement.shader_name,
                                    Some(
                                        replacement.path.as_path()
                                    ),
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
                            bloom_selection_from_mode(
                                live_postprocess_profile.bloom
                            ),
                            live_postprocess_profile.bloom_intensity,
                            live_postprocess_profile.bloom_threshold,
                            live_postprocess_profile.invert_colors,
                            live_postprocess_profile.flip_horizontal,
                            live_postprocess_profile.flip_vertical,
                            live_postprocess_profile.hue_rotation,
                            active.texture_manager
                                .active_specification_selection(),
                            new_policy_exists,
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
                            bloom_selection_from_mode(
                                live_postprocess_profile.bloom
                            ),
                            live_postprocess_profile.bloom_intensity,
                            live_postprocess_profile.bloom_threshold,
                            live_postprocess_profile.invert_colors,
                            live_postprocess_profile.flip_horizontal,
                            live_postprocess_profile.flip_vertical,
                            live_postprocess_profile.hue_rotation,
                            active.texture_manager
                                .active_specification_selection(),
                            match editor_output.policy_target {
                                Some(
                                    crate::editor_layout::PolicyTarget::Screensaver
                                ) => {
                                    screensaver_policy_exists
                                }

                                Some(
                                    crate::editor_layout::PolicyTarget::Wallpaper
                                ) => {
                                    wallpaper_policy_exists
                                }

                                Some(
                                    crate::editor_layout::PolicyTarget::Unassigned
                                ) => {
                                    config.unassigned_policies
                                        .iter()
                                        .any(
                                            |policy| {
                                                policy_applies_to_path(
                                                    policy,
                                                    crate::editor_layout::PolicyTarget::Unassigned,
                                                    &active.path,
                                                )
                                            }
                                        )
                                }

                                None => {
                                    false
                                }
                            },
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

                        crate::editor_layout::PolicyTarget::Unassigned => {
                            true
                        }
                    };

                if !target_available {
                    edit_window.set_status_message(
                        match requested_target {
                            crate::editor_layout::PolicyTarget::Screensaver => {
                                "This shader cannot use a Screensaver policy in the current editing session."
                            }

                            crate::editor_layout::PolicyTarget::Wallpaper => {
                                "This shader cannot use a Wallpaper policy in the current editing session."
                            }

                            crate::editor_layout::PolicyTarget::Unassigned => {
                                "This shader cannot use an Unassigned policy in the current editing session."
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

                        crate::editor_layout::PolicyTarget::Unassigned => {
                            config.unassigned_policies
                                .iter()
                                .any(
                                    |policy| {
                                        policy_applies_to_path(
                                            policy,
                                            crate::editor_layout::PolicyTarget::Unassigned,
                                            &active.path,
                                        )
                                    }
                                )
                        }
                    };

                (
                    global_rendered_fps,
                    fps_policy_entries,
                    texture_policy,
                    postprocess_policy,
                    animation_speed,
                ) =
                    editor_policy_context_for_path(
                        &config,
                        Some(
                            requested_target
                        ),
                        &active.path,
                        edit_window
                            .policy_list_state_snapshot()
                            .selected_policy_row
                            .as_ref()
                            .map(
                                |row| row.policy_id
                            ),
                        edit_window
                            .policy_list_state_snapshot()
                            .selected_policy_row
                            .as_ref()
                            .map(
                                |row| row.policy_key.as_str()
                            ),
                        command_line_animation_speed,
                    );


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
                        &active.shader_name,
                        Some(
                            active.path.as_path()
                        ),
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

                        crate::editor_layout::PolicyTarget::Unassigned => {
                            "Unassigned"
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
                    bloom_selection_from_mode(
                        live_postprocess_profile.bloom
                    ),
                    live_postprocess_profile.bloom_intensity,
                    live_postprocess_profile.bloom_threshold,
                    live_postprocess_profile.invert_colors,
                    live_postprocess_profile.flip_horizontal,
                    live_postprocess_profile.flip_vertical,
                    live_postprocess_profile.hue_rotation,
                    active.texture_manager
                        .active_specification_selection(),
                    target_policy_exists,
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

            let selected_bloom_mode =
                bloom_mode_from_selection(
                    editor_output.bloom
                );

            let selected_bloom_intensity =
                editor_output.bloom_intensity;

            let selected_bloom_threshold =
                editor_output.bloom_threshold;

            let selected_invert_colors =
                editor_output.invert_colors;

            let selected_flip_horizontal =
                editor_output.flip_horizontal;

            let selected_flip_vertical =
                editor_output.flip_vertical;

            let selected_hue_rotation =
                editor_output.hue_rotation;


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
                || selected_bloom_mode
                    != live_postprocess_profile.bloom
                || (selected_bloom_intensity
                    - live_postprocess_profile.bloom_intensity)
                    .abs()
                    > f32::EPSILON
                || (selected_bloom_threshold
                    - live_postprocess_profile.bloom_threshold)
                    .abs()
                    > f32::EPSILON
                || selected_invert_colors
                    != live_postprocess_profile.invert_colors
                || selected_flip_horizontal
                    != live_postprocess_profile.flip_horizontal
                || selected_flip_vertical
                    != live_postprocess_profile.flip_vertical
                || (selected_hue_rotation
                    - live_postprocess_profile.hue_rotation)
                    .abs()
                    > f32::EPSILON
            {
                live_postprocess_profile.render_scale =
                    selected_render_scale;

                live_postprocess_profile.anti_aliasing =
                    selected_anti_aliasing_method;

                live_postprocess_profile.dithering =
                    selected_dithering_level;

                live_postprocess_profile.color_precision =
                    selected_color_precision_policy;

                live_postprocess_profile.bloom =
                    selected_bloom_mode;

                live_postprocess_profile.bloom_intensity =
                    selected_bloom_intensity;

                live_postprocess_profile.bloom_threshold =
                    selected_bloom_threshold;
                live_postprocess_profile.invert_colors =
                    selected_invert_colors;

                live_postprocess_profile.flip_horizontal =
                    selected_flip_horizontal;

                live_postprocess_profile.flip_vertical =
                    selected_flip_vertical;

                live_postprocess_profile.hue_rotation =
                    selected_hue_rotation;

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


            if editor_output.bulk_save_requested {
            log_information(
                &format!(
                    "[EDIT_SHADER] Confirmed Bulk Edit save request received: selected_policies={}, changes={:?}",
                    editor_output.bulk_selected_policy_rows.len(),
                    editor_output.bulk_edit_changes,
                )
            );

            if !editor_output.bulk_edit_changes.any() {
                log_warning(
                    "[EDIT_SHADER] Bulk Edit save request contained an empty field-change mask; no database update was attempted"
                );
                edit_window.set_status_message(
                    "Bulk Edit contains no changed settings."
                );
                continue;
            }

            let mut patches =
                Vec::with_capacity(
                    editor_output
                        .bulk_selected_policy_rows
                        .len()
                );

            let mut preparation_error:
                Option<String> =
                None;

            for row in
                &editor_output.bulk_selected_policy_rows
            {
                let shader_path =
                    PathBuf::from(
                        &row.full_path
                    );

                let texture_fields_changed =
                    editor_output.bulk_edit_changes.texture
                        || editor_output.bulk_edit_changes.palette
                        || editor_output.bulk_edit_changes.primitive_count;

                let texture_required =
                    if texture_fields_changed {
                        match shader_requires_texture_for_bulk_edit(
                            &shader_path
                        ) {
                            Ok(required) => required,
                            Err(error) => {
                                preparation_error =
                                    Some(error);
                                break;
                            }
                        }
                    } else {
                        false
                    };

                patches.push(
                    bulk_policy_patch_from_editor_output(
                        row,
                        &editor_output,
                        texture_required,
                    )
                );
            }

            if let Some(error) =
                preparation_error
            {
                edit_window.set_status_message(
                    format!(
                        "Bulk policy save aborted: {}",
                        error,
                    )
                );

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Bulk policy save aborted before database transaction: {}",
                        error,
                    )
                );

                continue;
            }

            let protected_target_skips =
                protected_bulk_target_skip_count(
                    &patches,
                    &editor_output.bulk_selected_policy_rows,
                );

            match crate::manage_policies::patch_policies_by_id(
                &patches
            ) {
                Ok(changed) => {
                    let config_path =
                        crate::locate_paths::config_path();

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

                            edit_window.complete_bulk_save(
                                false
                            );

                            edit_window.set_status_message(
                                bulk_edit_completion_message(
                                    changed,
                                    protected_target_skips,
                                )
                            );

                            log_information(
                                &format!(
                                    "[EDIT_SHADER] Bulk Edit updated {} policies in one database transaction",
                                    changed,
                                )
                            );

                            if editor_output.exit_after_save_requested {
                                break 'preview Ok(());
                            }
                        }

                        Err(error) => {
                            edit_window.set_status_message(
                                "Bulk policies were saved, but configuration reload failed."
                            );

                            log_warning(
                                &format!(
                                    "[EDIT_SHADER] Bulk policies were saved, but configuration reload failed: {}",
                                    error,
                                )
                            );
                        }
                    }
                }

                Err(error) => {
                    edit_window.set_status_message(
                        format!(
                            "Unable to save bulk policy changes: {}",
                            error,
                        )
                    );

                    log_warning(
                        &format!(
                            "[EDIT_SHADER] Bulk policy save failed: {}",
                            error,
                        )
                    );
                }
            }

            continue;
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

                        crate::editor_layout::PolicyTarget::Unassigned => {
                            true
                        }
                    };

                if !selected_target_available {
                    edit_window.set_status_message(
                        "The selected policy target is unavailable in the current editing session."
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
                                .palette()
                                .to_hex()
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

                        bloom:
                            Some(
                                live_postprocess_profile
                                    .bloom
                                    .name()
                                    .to_string()
                            ),

                        bloom_intensity:
                            Some(
                                live_postprocess_profile
                                    .bloom_intensity
                            ),

                        bloom_threshold:
                            Some(
                                live_postprocess_profile
                                    .bloom_threshold
                            ),

                        invert_colors:
                            Some(live_postprocess_profile.invert_colors),

                        flip_horizontal:
                            Some(live_postprocess_profile.flip_horizontal),

                        flip_vertical:
                            Some(live_postprocess_profile.flip_vertical),

                        hue_rotation:
                            Some(live_postprocess_profile.hue_rotation),
                    };

                let manage_target =
                    match policy_target {
                        crate::editor_layout::PolicyTarget::Screensaver => {
                            crate::manage_policies::PolicyTarget::Screensaver
                        }

                        crate::editor_layout::PolicyTarget::Wallpaper => {
                            crate::manage_policies::PolicyTarget::Wallpaper
                        }

                        crate::editor_layout::PolicyTarget::Unassigned => {
                            crate::manage_policies::PolicyTarget::Unassigned
                        }
                    };

                let config_path =
                    crate::locate_paths::config_path();

                // Policies belong to the shader copy for the selected runtime
                // target, not necessarily to the copy that is currently active
                // in the editor. A shader opened from the wallpaper directory
                // may also have a screensaver copy (and vice versa). Using
                // active.path here incorrectly associates the second policy
                // with the first target's file, causing that runtime to miss the
                // policy and fall back to its global texture/palette defaults.
                let policy_source_path =
                    if policy_target
                        == crate::editor_layout::PolicyTarget::Unassigned
                    {
                        active.path.clone()
                    } else {
                        match managed_policy_target_for_path(
                            &active.path
                        ) {
                            Some(
                                managed_target
                            ) => {
                                target_shader_path(
                                    managed_target,
                                    &active.shader_name,
                                )
                            }

                            None => {
                                active.path.clone()
                            }
                        }
                    };

                let selected_policy_before_save =
                    edit_window
                        .active_selected_policy_row();


                let retarget_result =
                    if let Some(
                        selected_policy
                    ) =
                        selected_policy_before_save
                            .as_ref()
                    {
                        let selected_manage_target =
                            match selected_policy.policy_target {

                                crate::editor_layout::PolicyTarget::Screensaver => {
                                    crate::manage_policies::PolicyTarget::Screensaver
                                }

                                crate::editor_layout::PolicyTarget::Wallpaper => {
                                    crate::manage_policies::PolicyTarget::Wallpaper
                                }

                                crate::editor_layout::PolicyTarget::Unassigned => {
                                    crate::manage_policies::PolicyTarget::Unassigned
                                }
                            };


                        if selected_manage_target
                            != manage_target
                        {
                            crate::manage_policies::retarget_policy_by_id(
                                selected_policy.policy_id,
                                manage_target,
                            )
                            .map(
                                |_| {
                                    true
                                }
                            )
                        } else {
                            Ok(
                                false
                            )
                        }
                    } else {
                        Ok(
                            false
                        )
                    };


                let save_result =
                    match retarget_result {

                        Err(error) => {
                            Err(
                                error
                            )
                        }

                        Ok(
                            _retargeted
                        ) => {
                            if crate::manage_policies::policy_exists_for_source(
                        &config_path,
                        manage_target,
                        &active.shader_name,
                        &policy_source_path,
                    )? {
                        if let Some(
                            selected_policy
                        ) =
                            selected_policy_before_save
                                .as_ref()
                        {
                            crate::manage_policies::replace_policy_by_id(
                                selected_policy.policy_id,
                                properties,
                            )
                        } else {
                            crate::manage_policies::replace_policy_for_source(
                                &config_path,
                                manage_target,
                                &active.shader_name,
                                properties,
                                &policy_source_path,
                            )
                        }
                    } else {
                        crate::manage_policies::add_policy_for_source(
                            &config_path,
                            manage_target,
                            &active.shader_name,
                            properties,
                            &policy_source_path,
                        )
                            }
                        }
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

                                let screensaver_policy_path =
                                    target_shader_path(
                                        crate::editor_layout::PolicyTarget::Screensaver,
                                        &active.shader_name,
                                    );

                                screensaver_policy_exists =
                                    screensaver_target_available
                                        && config.screensaver_policies
                                        .iter()
                                        .any(
                                            |policy| {
                                                policy_applies_to_path(
                                                    policy,
                                                    crate::editor_layout::PolicyTarget::Screensaver,
                                                    &screensaver_policy_path,
                                                )
                                            }
                                        );

                                let wallpaper_policy_path =
                                    target_shader_path(
                                        crate::editor_layout::PolicyTarget::Wallpaper,
                                        &active.shader_name,
                                    );

                                wallpaper_policy_exists =
                                    wallpaper_target_available
                                        && config.wallpaper_policies
                                        .iter()
                                        .any(
                                            |policy| {
                                                policy_applies_to_path(
                                                    policy,
                                                    crate::editor_layout::PolicyTarget::Wallpaper,
                                                    &wallpaper_policy_path,
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
                        if editor_output.exit_after_save_requested {
                            exit_save_failed =
                                true;
                        }

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


            if editor_output.exit_after_save_requested
                && !exit_save_failed
                && !editor_output.bulk_save_requested
            {
                break 'preview Ok(());
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
                    crate::editor_layout::PolicyRowCommand::MoveToScreensavers
                    | crate::editor_layout::PolicyRowCommand::MoveToWallpapers => {
                        let Some(destination_target) =
                            policy_move_destination(
                                *command
                            )
                        else {
                            unreachable!();
                        };

                        match move_policy_shader(
                            &config,
                            row,
                            destination_target,
                        ) {
                            Ok(destination_path) => {
                                match crate::load_config::load_config(
                                    &crate::locate_paths::config_path()
                                ) {
                                    Ok(reloaded_config) => {
                                        config =
                                            reloaded_config.config;

                                        policy_display_rows =
                                            build_policy_display_rows(
                                                &config
                                            );


                                        // The shader has physically moved into
                                        // a managed runtime folder.  Adopt the
                                        // destination as the authoritative
                                        // active path and policy target, and
                                        // re-baseline the editor so this
                                        // administrative move does not appear
                                        // as an unsaved user edit.
                                        active.path =
                                            destination_path.clone();

                                        information_path =
                                            destination_path.clone();


                                        match destination_target {
                                            crate::editor_layout::PolicyTarget::Screensaver => {
                                                screensaver_target_available =
                                                    true;

                                                wallpaper_target_available =
                                                    false;

                                                screensaver_policy_exists =
                                                    true;

                                                wallpaper_policy_exists =
                                                    false;
                                            }

                                            crate::editor_layout::PolicyTarget::Wallpaper => {
                                                screensaver_target_available =
                                                    false;

                                                wallpaper_target_available =
                                                    true;

                                                screensaver_policy_exists =
                                                    false;

                                                wallpaper_policy_exists =
                                                    true;
                                            }

                                            crate::editor_layout::PolicyTarget::Unassigned => {
                                                screensaver_target_available =
                                                    true;

                                                wallpaper_target_available =
                                                    true;

                                                screensaver_policy_exists =
                                                    false;

                                                wallpaper_policy_exists =
                                                    false;
                                            }
                                        }


                                        edit_window.initialize_configuration(
                                            configured_fps,
                                            animation_speed,
                                            render_scale,
                                            Some(
                                                destination_target
                                            ),
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
                                            bloom_selection_from_mode(
                                                live_postprocess_profile
                                                    .bloom
                                            ),
                                            live_postprocess_profile
                                                .bloom_intensity,
                                            live_postprocess_profile
                                                .bloom_threshold,
                                            live_postprocess_profile
                                                .invert_colors,
                                            live_postprocess_profile
                                                .flip_horizontal,
                                            live_postprocess_profile
                                                .flip_vertical,
                                            live_postprocess_profile
                                                .hue_rotation,
                                            active.texture_manager
                                                .active_specification_selection(),
                                            true,
                                            format!(
                                                "Shader moved to {}. Policy target updated to {}.",
                                                destination_path
                                                    .parent()
                                                    .map(
                                                        |path| path.display().to_string()
                                                    )
                                                    .unwrap_or_default(),
                                                match destination_target {
                                                    crate::editor_layout::PolicyTarget::Screensaver =>
                                                        "Screensaver",

                                                    crate::editor_layout::PolicyTarget::Wallpaper =>
                                                        "Wallpaper",

                                                    crate::editor_layout::PolicyTarget::Unassigned =>
                                                        "Unassigned",
                                                },
                                            ),
                                        );


                                        edit_window.set_status_message(
                                            format!(
                                                "Shader moved to {}.",
                                                destination_path
                                                    .parent()
                                                    .map(
                                                        |path| path.display().to_string()
                                                    )
                                                    .unwrap_or_default(),
                                            )
                                        );

                                        log_information(
                                            &format!(
                                                "[EDIT_SHADER] Moved shader {} to {}",
                                                row.filename,
                                                destination_path.display(),
                                            )
                                        );
                                    }

                                    Err(error) => {
                                        edit_window.set_status_message(
                                            "Shader moved; configuration reload failed."
                                        );

                                        log_warning(
                                            &format!(
                                                "[EDIT_SHADER] Shader moved, but configuration reload failed: {}",
                                                error,
                                            )
                                        );
                                    }
                                }
                            }

                            Err(error) => {
                                edit_window.set_status_message(
                                    error.clone()
                                );

                                log_warning(
                                    &format!(
                                        "[EDIT_SHADER] Unable to move shader {}: {}",
                                        row.filename,
                                        error,
                                    )
                                );
                            }
                        }

                        continue;
                    }

                    crate::editor_layout::PolicyRowCommand::DeletePolicy => {
                        let manage_target =
                            match row.policy_target {
                                crate::editor_layout::PolicyTarget::Screensaver => {
                                    crate::manage_policies::PolicyTarget::Screensaver
                                }

                                crate::editor_layout::PolicyTarget::Wallpaper => {
                                    crate::manage_policies::PolicyTarget::Wallpaper
                                }

                                crate::editor_layout::PolicyTarget::Unassigned => {
                                    crate::manage_policies::PolicyTarget::Unassigned
                                }
                            };

                        let config_path =
                            crate::locate_paths::config_path();

                        match crate::manage_policies::delete_policy_by_id(
                &config_path,
                row.policy_id,
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

                                crate::editor_layout::PolicyTarget::Unassigned => {
                                    crate::manage_policies::PolicyTarget::Unassigned
                                }
                            };

                        let shader_path =
                            PathBuf::from(
                                &row.full_path
                            );

                        let config_path =
                            crate::locate_paths::config_path();

                        let policy_delete_result =
                            crate::manage_policies::delete_policy_by_id(
                &config_path,
                row.policy_id,
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

                    crate::editor_layout::PolicyRowCommand::ClonePolicy => {
                        // Clone Policy is handled by process_policy_clone_ui().
                    }

                    crate::editor_layout::PolicyRowCommand::RenamePolicy => {
                        // Rename Policy is handled by process_policy_rename_ui().
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

            crate::editor_layout::PolicyTarget::Unassigned => {
                &config.unassigned_policies
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



fn editor_policy_context_for_path(
    config: &crate::load_config::Config,
    target: Option<crate::editor_layout::PolicyTarget>,
    loaded_path: &Path,
    selected_policy_id: Option<i64>,
    selected_policy_name: Option<&str>,
    command_line_animation_speed: Option<f32>,
) -> (
    u32,
    Vec<crate::load_config::FpsPolicyEntry>,
    crate::load_config::TexturePolicy,
    crate::load_config::PostprocessPolicy,
    f32,
) {

    let (
        global_rendered_fps,
        base_texture_policy,
        base_postprocess_policy,
        global_animation_speed,
        policies,
    ) =
        match target {

            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ) => (
                config.wallpaper_fps_policy
                    .global_rendered_fps,
                &config.wallpaper_texture_policy,
                &config.wallpaper_postprocess_policy,
                config.wallpaper_speed_policy
                    .global_speed,
                &config.wallpaper_policies,
            ),

            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ) => (
                config.global_rendered_fps,
                &config.texture_policy,
                &config.screensaver_postprocess_policy,
                config.screensaver_speed_policy
                    .global_speed,
                &config.screensaver_policies,
            ),

            Some(
                crate::editor_layout::PolicyTarget::Unassigned
            ) => (
                config.global_rendered_fps,
                &config.texture_policy,
                &config.screensaver_postprocess_policy,
                config.screensaver_speed_policy
                    .global_speed,
                &config.unassigned_policies,
            ),

            None => (
                config.global_rendered_fps,
                &config.texture_policy,
                &config.screensaver_postprocess_policy,
                config.screensaver_speed_policy
                    .global_speed,
                &config.screensaver_policies,
            ),
        };


    let matching_policy =
        target.and_then(
            |resolved_target| {
                selected_policy_id
                    .and_then(
                        |selected_id| {
                            policies
                                .iter()
                                .find(
                                    |policy| {
                                        policy.policy_id
                                            == selected_id
                                    }
                                )
                        }
                    )
                    .or_else(
                        || {
                            if selected_policy_id.is_some() {
                                None
                            } else {
                                selected_policy_name
                                    .and_then(
                                        |selected_name| {
                                            policies
                                                .iter()
                                                .find(
                                                    |policy| {
                                                        policy.policy_key
                                                            .eq_ignore_ascii_case(
                                                                selected_name
                                                            )
                                                    }
                                                )
                                        }
                                    )
                            }
                        }
                    )
                    .or_else(
                        || {
                            if selected_policy_id.is_some() {
                                None
                            } else {
                                policies
                                    .iter()
                                    .find(
                                        |policy| {
                                            policy_applies_to_path(
                                                policy,
                                                resolved_target,
                                                loaded_path,
                                            )
                                        }
                                    )
                            }
                        }
                    )
            }
        );


    let fps_policy_entries =
        matching_policy
            .and_then(
                |policy| {
                    policy.rendered_fps
                        .map(
                            |rendered_fps| {
                                crate::load_config::FpsPolicyEntry {
                                    policy_id:
                                        policy.policy_id,
                                    shader:
                                        policy.shader.clone(),
                                    source_path:
                                        policy.source_path.clone(),
                                    rendered_fps,
                                }
                            }
                        )
                }
            )
            .into_iter()
            .collect();


    let texture_policy_entries =
        matching_policy
            .filter(
                |policy| {
                    policy.shader_texture.is_some()
                        || policy.shader_palette.is_some()
                }
            )
            .map(
                |policy| {
                    crate::load_config::TexturePolicyEntry {
                        policy_id:
                            policy.policy_id,
                        shader:
                            policy.shader.clone(),
                        source_path:
                            policy.source_path.clone(),
                        shader_texture:
                            policy.shader_texture.clone(),
                        shader_palette:
                            policy.shader_palette,
                    }
                }
            )
            .into_iter()
            .collect();


    let texture_policy =
        crate::load_config::TexturePolicy {
            global_texture:
                base_texture_policy
                    .global_texture
                    .clone(),
            global_palette:
                base_texture_policy
                    .global_palette,
            texture_policy_entries,
        };


    let postprocess_policy =
        crate::load_config::PostprocessPolicy {
            global_profile:
                base_postprocess_policy
                    .global_profile,
            shader_policies:
                matching_policy
                    .cloned()
                    .into_iter()
                    .collect(),
        };


    let animation_speed =
        command_line_animation_speed
            .or_else(
                || {
                    matching_policy
                        .and_then(
                            |policy| {
                                policy.animation_speed
                            }
                        )
                }
            )
            .unwrap_or(
                global_animation_speed
            );


    (
        global_rendered_fps,
        fps_policy_entries,
        texture_policy,
        postprocess_policy,
        animation_speed,
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



fn move_policy_shader(
    config: &crate::load_config::Config,
    row: &crate::editor_layout::PolicyRowReference,
    destination_target: crate::editor_layout::PolicyTarget,
) -> Result<PathBuf, String> {

    let source_path =
        PathBuf::from(
            &row.full_path
        );

    if !source_path.is_file() {
        return Err(
            format!(
                "Shader file is unavailable: {}",
                source_path.display(),
            )
        );
    }

    let destination_directory =
        match destination_target {
            crate::editor_layout::PolicyTarget::Screensaver =>
                crate::locate_paths::shader_dir(),

            crate::editor_layout::PolicyTarget::Wallpaper =>
                crate::locate_paths::shader_dir(),

            crate::editor_layout::PolicyTarget::Unassigned =>
                crate::locate_paths::shader_dir(),
        };

    std::fs::create_dir_all(
        &destination_directory
    )
    .map_err(
        |error| {
            format!(
                "Unable to create destination directory {} ({})",
                destination_directory.display(),
                error,
            )
        }
    )?;

    let destination_path =
        destination_directory
            .join(
                &row.filename
            );

    if source_path == destination_path {
        return Ok(
            destination_path
        );
    }

    if destination_path.exists() {
        return Err(
            format!(
                "Shader already exists in {}.",
                match destination_target {
                    crate::editor_layout::PolicyTarget::Screensaver =>
                        "/screensavers",

                    crate::editor_layout::PolicyTarget::Wallpaper =>
                        "/wallpapers",

                    crate::editor_layout::PolicyTarget::Unassigned =>
                        "/shaders",
                }
            )
        );
    }

    std::fs::rename(
        &source_path,
        &destination_path,
    )
    .map_err(
        |error| {
            format!(
                "Unable to move shader from {} to {} ({})",
                source_path.display(),
                destination_path.display(),
                error,
            )
        }
    )?;

    let config_path =
        crate::locate_paths::config_path();

    if let Err(error) =
        crate::manage_policies::reconcile_shader_move_from_source(
            &config_path,
            &source_path,
            &destination_path,
            match destination_target {
                crate::editor_layout::PolicyTarget::Screensaver =>
                    crate::manage_policies::PolicyTarget::Screensaver,

                crate::editor_layout::PolicyTarget::Wallpaper =>
                    crate::manage_policies::PolicyTarget::Wallpaper,

                crate::editor_layout::PolicyTarget::Unassigned =>
                    crate::manage_policies::PolicyTarget::Unassigned,
            },
        )
    {
        let rollback_result =
            std::fs::rename(
                &destination_path,
                &source_path,
            );

        return Err(
            match rollback_result {
                Ok(()) =>
                    format!(
                        "Shader move was rolled back because policy paths could not be updated: {}",
                        error,
                    ),

                Err(rollback_error) =>
                    format!(
                        "Policy paths could not be updated after moving the shader: {}. Rollback also failed: {}",
                        error,
                        rollback_error,
                    ),
            }
        );
    }

    Ok(
        destination_path
    )
}


fn policy_move_destination(
    command: crate::editor_layout::PolicyRowCommand,
) -> Option<crate::editor_layout::PolicyTarget> {

    match command {
        crate::editor_layout::PolicyRowCommand::MoveToScreensavers =>
            Some(
                crate::editor_layout::PolicyTarget::Screensaver
            ),

        crate::editor_layout::PolicyRowCommand::MoveToWallpapers =>
            Some(
                crate::editor_layout::PolicyTarget::Wallpaper
            ),

        _ =>
            None,
    }
}


fn analyze_bulk_policy_candidates(
    shader_paths: Vec<PathBuf>,
) -> (
    Vec<crate::editor_layout::BulkCreateCandidate>,
    usize,
) {

    let mut candidates =
        Vec::with_capacity(
            shader_paths.len()
        );

    let mut rejected_count =
        0usize;


    for shader_path in shader_paths {

        if !shader_path.is_file() {
            rejected_count +=
                1;

            log_warning(
                &format!(
                    "[EDIT_SHADER] Bulk policy creation skipped unavailable shader: {}",
                    shader_path.display(),
                )
            );

            continue;
        }


        match crate::load_shader::load_shader_for_preview(
            &shader_path
        ) {
            crate::load_shader::ShaderLoadResult::Ready {
                channel_usage,
                ..
            } => {
                candidates.push(
                    crate::editor_layout::BulkCreateCandidate {
                        forced_target:
                            managed_policy_target_for_path(
                                &shader_path
                            ),

                        texture_required:
                            channel_usage
                                .uses_any_channel(),

                        path:
                            shader_path,
                    }
                );
            }

            crate::load_shader::ShaderLoadResult::Rejected {
                shader_name,
                reasons,
            } => {
                rejected_count +=
                    1;

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Bulk policy creation skipped rejected shader '{}': {}",
                        shader_name,
                        reasons.join("; "),
                    )
                );
            }

            crate::load_shader::ShaderLoadResult::Unavailable {
                shader_name,
                error,
            } => {
                rejected_count +=
                    1;

                log_warning(
                    &format!(
                        "[EDIT_SHADER] Bulk policy creation skipped unavailable shader '{}': {}",
                        shader_name,
                        error,
                    )
                );
            }
        }
    }


    (
        candidates,
        rejected_count,
    )
}


fn bulk_policy_patch_from_editor_output(
    row: &crate::editor_layout::PolicyRowReference,
    editor_output: &crate::editor_layout::EditorOutput,
    texture_required: bool,
) -> crate::manage_policies::BulkPolicyPatch {

    let changes =
        editor_output.bulk_edit_changes;

    let texture =
        if changes.texture || changes.primitive_count {
            if texture_required {
                Some(
                    format!(
                        "{}:{}",
                        editor_output.texture.name(),
                        editor_output.primitive_count,
                    )
                )
            } else {
                None
            }
        } else {
            None
        };

    let palette =
        if changes.palette && texture_required {
            Some(
                editor_output.palette
                    .palette()
                    .to_hex()
            )
        } else {
            None
        };

    let anti_aliasing =
        if changes.anti_aliasing {
            Some(
                match editor_output.anti_aliasing {
                    crate::editor_layout::AntiAliasingSelection::Off => "off",
                    crate::editor_layout::AntiAliasingSelection::Fxaa => "fxaa",
                }
                .to_string()
            )
        } else {
            None
        };

    let dithering =
        if changes.dithering {
            Some(
                match editor_output.dithering {
                    crate::editor_layout::DitheringSelection::Off => "off",
                    crate::editor_layout::DitheringSelection::Subtle => "subtle",
                }
                .to_string()
            )
        } else {
            None
        };

    let color_precision =
        if changes.color_precision {
            Some(
                match editor_output.color_precision {
                    crate::editor_layout::ColorPrecisionSelection::Automatic => "auto",
                    crate::editor_layout::ColorPrecisionSelection::High => "high",
                    crate::editor_layout::ColorPrecisionSelection::Standard => "standard",
                }
                .to_string()
            )
        } else {
            None
        };

    let current_target =
        match row.policy_target {
            crate::editor_layout::PolicyTarget::Screensaver =>
                crate::manage_policies::PolicyTarget::Screensaver,
            crate::editor_layout::PolicyTarget::Wallpaper =>
                crate::manage_policies::PolicyTarget::Wallpaper,
            crate::editor_layout::PolicyTarget::Unassigned =>
                crate::manage_policies::PolicyTarget::Unassigned,
        };

    let destination_target =
        if changes.policy_target {
            editor_output
                .policy_target
                .map(
                    |target| {
                        match target {
                            crate::editor_layout::PolicyTarget::Screensaver =>
                                crate::manage_policies::PolicyTarget::Screensaver,
                            crate::editor_layout::PolicyTarget::Wallpaper =>
                                crate::manage_policies::PolicyTarget::Wallpaper,
                            crate::editor_layout::PolicyTarget::Unassigned =>
                                crate::manage_policies::PolicyTarget::Unassigned,
                        }
                    }
                )
        } else {
            None
        };

    crate::manage_policies::BulkPolicyPatch {
        policy_id:
            row.policy_id,
        current_target,
        destination_target,
        policy_key:
            row.policy_key.clone(),
        fields:
            crate::manage_policies::BulkPolicyFieldMask {
                policy_target:
                    changes.policy_target,
                texture:
                    texture_required
                        && (changes.texture || changes.primitive_count),
                palette:
                    texture_required
                        && changes.palette,
                fps:
                    changes.fps,
                speed:
                    changes.animation_speed,
                render_scale:
                    changes.render_scale,
                anti_aliasing:
                    changes.anti_aliasing,
                dithering:
                    changes.dithering,
                color_precision:
                    changes.color_precision,
                bloom:
                    changes.bloom,
                bloom_intensity:
                    changes.bloom_intensity,
                bloom_threshold:
                    changes.bloom_threshold,
                invert_colors:
                    changes.invert_colors,
                flip_horizontal:
                    changes.flip_horizontal,
                flip_vertical:
                    changes.flip_vertical,
                hue_rotation:
                    changes.hue_rotation,
            },
        properties:
            crate::manage_policies::PolicyDefinition {
                texture,
                palette,
                fps:
                    changes.fps.then_some(editor_output.fps),
                speed:
                    changes.animation_speed
                        .then_some(editor_output.animation_speed),
                render_scale:
                    changes.render_scale
                        .then_some(editor_output.render_scale),
                anti_aliasing,
                dithering,
                color_precision,
                bloom:
                    changes.bloom.then(
                        || {
                            bloom_mode_from_selection(
                                editor_output.bloom
                            )
                            .name()
                            .to_string()
                        }
                    ),
                bloom_intensity:
                    changes.bloom_intensity
                        .then_some(editor_output.bloom_intensity),
                bloom_threshold:
                    changes.bloom_threshold
                        .then_some(editor_output.bloom_threshold),
                invert_colors:
                    changes.invert_colors
                        .then_some(editor_output.invert_colors),
                flip_horizontal:
                    changes.flip_horizontal
                        .then_some(editor_output.flip_horizontal),
                flip_vertical:
                    changes.flip_vertical
                        .then_some(editor_output.flip_vertical),
                hue_rotation:
                    changes.hue_rotation
                        .then_some(editor_output.hue_rotation),
            },
    }
}


fn bulk_policy_definition_from_editor_output(
    editor_output: &crate::editor_layout::EditorOutput,
    texture_required: bool,
) -> crate::manage_policies::PolicyDefinition {

    let (
        texture,
        palette,
    ) =
        if texture_required {
            (
                Some(
                    format!(
                        "{}:{}",
                        editor_output.texture
                            .name(),
                        editor_output.primitive_count,
                    )
                ),
                Some(
                    editor_output.palette
                        .palette()
                        .to_hex()
                ),
            )
        } else {
            (
                None,
                None,
            )
        };


    let anti_aliasing =
        match editor_output.anti_aliasing {
            crate::editor_layout::AntiAliasingSelection::Off =>
                "off",

            crate::editor_layout::AntiAliasingSelection::Fxaa =>
                "fxaa",
        };


    let dithering =
        match editor_output.dithering {
            crate::editor_layout::DitheringSelection::Off =>
                "off",

            crate::editor_layout::DitheringSelection::Subtle =>
                "subtle",
        };


    let color_precision =
        match editor_output.color_precision {
            crate::editor_layout::ColorPrecisionSelection::Automatic =>
                "auto",

            crate::editor_layout::ColorPrecisionSelection::High =>
                "high",

            crate::editor_layout::ColorPrecisionSelection::Standard =>
                "standard",
        };


    crate::manage_policies::PolicyDefinition {
        texture,

        palette,

        fps:
            Some(
                editor_output.fps
            ),

        speed:
            Some(
                editor_output.animation_speed
            ),

        render_scale:
            Some(
                editor_output.render_scale
            ),

        anti_aliasing:
            Some(
                anti_aliasing.to_string()
            ),

        dithering:
            Some(
                dithering.to_string()
            ),

        color_precision:
            Some(
                color_precision.to_string()
            ),

        bloom:
            Some(
                bloom_mode_from_selection(
                    editor_output.bloom
                )
                .name()
                .to_string()
            ),

        bloom_intensity:
            Some(
                editor_output.bloom_intensity
            ),

        bloom_threshold:
            Some(
                editor_output.bloom_threshold
            ),

        invert_colors:
            Some(editor_output.invert_colors),

        flip_horizontal:
            Some(editor_output.flip_horizontal),

        flip_vertical:
            Some(editor_output.flip_vertical),

        hue_rotation:
            Some(editor_output.hue_rotation),
    }
}


fn create_bulk_policies(
    request: &crate::editor_layout::BulkCreateRequest,
    editor_output: &crate::editor_layout::EditorOutput,
) -> Result<crate::manage_policies::BulkPolicyCreationResult, String> {

    let mut creations =
        Vec::with_capacity(
            request.candidates.len()
        );


    for candidate in
        &request.candidates
    {
        let editor_target =
            candidate.forced_target
                .or(
                    request.external_target
                )
                .ok_or_else(
                    || {
                        format!(
                            "No policy target was selected for external shader {}",
                            candidate.path.display(),
                        )
                    }
                )?;


        let target =
            match editor_target {
                crate::editor_layout::PolicyTarget::Screensaver => {
                    crate::manage_policies::PolicyTarget::Screensaver
                }

                crate::editor_layout::PolicyTarget::Wallpaper => {
                    crate::manage_policies::PolicyTarget::Wallpaper
                }

                crate::editor_layout::PolicyTarget::Unassigned => {
                    crate::manage_policies::PolicyTarget::Unassigned
                }
            };


        let shader =
            candidate.path
                .file_name()
                .and_then(
                    |name| {
                        name.to_str()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Shader filename is not valid UTF-8: {}",
                            candidate.path.display(),
                        )
                    }
                )?
                .to_string();


        creations.push(
            crate::manage_policies::BulkPolicyCreation {
                target,

                shader,

                source_path:
                    candidate.path.clone(),

                properties:
                    bulk_policy_definition_from_editor_output(
                        editor_output,
                        candidate.texture_required,
                    ),
            }
        );
    }


    crate::manage_policies::add_policies_for_sources(
        &crate::locate_paths::config_path(),
        &creations,
    )
}


fn managed_policy_target_for_path(
    _shader_path: &Path,
) -> Option<crate::editor_layout::PolicyTarget> {

    // A shader's physical location no longer determines its policy target.
    // Files in the canonical shader library may have Screensaver, Wallpaper,
    // both, or no policies. External files are likewise target-neutral.
    None
}


fn path_is_within_directory(
    candidate_path: &Path,
    managed_directory: &Path,
) -> bool {

    // Prefer filesystem-resolved paths so symlinks and equivalent relative
    // spellings cannot defeat managed-folder policy enforcement.
    if let (
        Ok(candidate_path),
        Ok(managed_directory),
    ) = (
        candidate_path.canonicalize(),
        managed_directory.canonicalize(),
    ) {
        return candidate_path
            .starts_with(
                &managed_directory
            );
    }


    // Fall back to lexical membership when canonicalization is unavailable.
    // This still handles direct files and any future subdirectories beneath a
    // managed shader directory.
    candidate_path
        .starts_with(
            managed_directory
        )
}


fn target_shader_path(
    _target: crate::editor_layout::PolicyTarget,
    shader_name: &str,
) -> PathBuf {

    crate::locate_paths::shader_dir()
        .join(
            shader_name
        )
}


fn resolve_information_path(
    loaded_path: &Path,
    shader_name: &str,
    _policy_target: Option<crate::editor_layout::PolicyTarget>,
) -> PathBuf {

    let managed_path =
        crate::locate_paths::shader_dir()
            .join(
                shader_name
            );


    if loaded_path.is_file() {
        loaded_path.to_path_buf()
    } else if managed_path.is_file() {
        managed_path
    } else {
        loaded_path.to_path_buf()
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


fn assign_selected_unassigned_policies(
    selected_rows: &[crate::editor_layout::PolicyRowReference],
    requested_target: Option<crate::editor_layout::PolicyTarget>,
) -> Result<Option<usize>, String> {

    if selected_rows.is_empty()
        || !selected_rows
            .iter()
            .all(
                |row| {
                    row.unassigned
                }
            )
    {
        return Ok(
            None
        );
    }


    let requested_target =
        requested_target
            .ok_or_else(
                || {
                    "Select Screensaver or Wallpaper as the Policy Target for the selected Unassigned policies."
                        .to_string()
                }
            )?;


    let destination_target =
        match requested_target {

            crate::editor_layout::PolicyTarget::Screensaver => {
                crate::manage_policies::PolicyTarget::Screensaver
            }

            crate::editor_layout::PolicyTarget::Wallpaper => {
                crate::manage_policies::PolicyTarget::Wallpaper
            }

            crate::editor_layout::PolicyTarget::Unassigned => {
                crate::manage_policies::PolicyTarget::Unassigned
            }
        };


    let policy_ids =
        selected_rows
            .iter()
            .map(
                |row| {
                    row.policy_id
                }
            )
            .collect::<Vec<_>>();


    crate::manage_policies::assign_unassigned_policies_by_id(
        &policy_ids,
        destination_target,
    )
    .map(
        Some
    )
}


fn policy_for_bulk_row<'a>(
    config: &'a crate::load_config::Config,
    row: &crate::editor_layout::PolicyRowReference,
) -> Option<&'a crate::load_config::ShaderPolicy> {

    let policies =
        match row.policy_target {
            crate::editor_layout::PolicyTarget::Screensaver => {
                &config.screensaver_policies
            }

            crate::editor_layout::PolicyTarget::Wallpaper => {
                &config.wallpaper_policies
            }

            crate::editor_layout::PolicyTarget::Unassigned => {
                &config.unassigned_policies
            }
        };


    policies
        .iter()
        .find(
            |policy| {
                policy.policy_id == row.policy_id
            }
        )
}


fn shader_requires_texture_for_bulk_edit(
    shader_path: &Path,
) -> Result<bool, String> {

    let filename =
        shader_path
            .file_name()
            .and_then(
                |name| name.to_str()
            )
            .ok_or_else(
                || {
                    format!(
                        "Shader path has no valid filename: {}",
                        shader_path.display(),
                    )
                }
            )?;


    let source_path =
        shader_path
            .parent()
            .unwrap_or_else(
                || Path::new(".")
            )
            .to_string_lossy()
            .to_string();


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while reading shader metadata for '{}': {}",
                        filename,
                        error,
                    )
                }
            )?;


    let row =
        connection
            .query_row(
                "SELECT
                     file_status,
                     validation_status,
                     validation_reason,
                     validation_message,
                     channel_usage_mask
                 FROM shaders
                 WHERE filename = ?1
                   AND source_path = ?2
                 LIMIT 1",
                rusqlite::params![
                    filename,
                    source_path,
                ],
                |row| {
                    Ok(
                        (
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<i64>>(4)?,
                        )
                    )
                },
            );


    let (
        file_status,
        validation_status,
        validation_reason,
        validation_message,
        channel_usage_mask,
    ) =
        match row {
            Ok(row) => row,

            Err(
                rusqlite::Error::QueryReturnedNoRows
            ) => {
                return Err(
                    format!(
                        "Shader '{}' is not registered in the Screenshaver database",
                        shader_path.display(),
                    )
                );
            }

            Err(error) => {
                return Err(
                    format!(
                        "Unable to read database metadata for '{}': {}",
                        shader_path.display(),
                        error,
                    )
                );
            }
        };


    if file_status != "present" {
        return Err(
            format!(
                "Shader '{}' has file_status '{}'",
                filename,
                file_status,
            )
        );
    }


    if validation_status == "rejected" {
        let reason =
            validation_message
                .or(
                    validation_reason
                )
                .unwrap_or_else(
                    || {
                        "shader is rejected"
                            .to_string()
                    }
                );

        return Err(
            format!(
                "Shader '{}' is rejected: {}",
                filename,
                reason,
            )
        );
    }


    if validation_status != "valid" {
        return Err(
            format!(
                "Shader '{}' has validation_status '{}'; expected 'valid'",
                filename,
                validation_status,
            )
        );
    }


    let mask =
        channel_usage_mask
            .ok_or_else(
                || {
                    format!(
                        "Valid shader '{}' has no channel-usage metadata",
                        filename,
                    )
                }
            )?;


    if !(0..=31).contains(
        &mask
    ) {
        return Err(
            format!(
                "Shader '{}' has invalid channel-usage mask {}",
                filename,
                mask,
            )
        );
    }


    // Bits 0-3 represent iChannel0-iChannel3. Bit 4 records the
    // mipmap requirement and does not, by itself, imply texture use.
    Ok(
        mask & 0x0f
            != 0
    )
}


fn shader_requires_texture_for_policy_row(
    shader_path: &Path,
) -> bool {

    match shader_requires_texture_for_bulk_edit(
        shader_path
    ) {
        Ok(required) => {
            required
        }

        Err(error) => {
            log_warning(
                &format!(
                    "[EDIT_SHADER] Policy-list texture requirement unavailable for {}: {}",
                    shader_path.display(),
                    error,
                )
            );

            false
        }
    }
}


fn build_policy_display_rows(
    _config: &crate::load_config::Config,
) -> Vec<crate::editor_layout::PolicyDisplayRow> {

    match load_database_policy_display_rows() {
        Ok(rows) => {
            rows
        }

        Err(error) => {
            log_warning(
                &format!(
                    "[EDIT_SHADER] Unable to load Policy List rows from database: {}",
                    error,
                )
            );

            Vec::new()
        }
    }
}


fn load_database_policy_display_rows(
) -> Result<Vec<crate::editor_layout::PolicyDisplayRow>, String> {

    let connection =
        crate::open_database::open()?;


    let mut statement =
        connection
            .prepare(
                "SELECT
                     p.policy_id,
                     p.policy_name,
                     s.filename,
                     s.source_path,
                     s.file_status,
                     p.policy_target
                 FROM shader_policies AS p
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 ORDER BY
                     p.policy_name_key,
                     p.policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare Policy List database query: {}",
                        error,
                    )
                }
            )?;


    let query_rows =
        statement
            .query_map(
                [],
                |row| {
                    Ok(
                        (
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                        )
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query Policy List rows from database: {}",
                        error,
                    )
                }
            )?;


    let mut display_rows =
        Vec::new();


    for query_row in query_rows {

        let (
            policy_id,
            policy_name,
            filename,
            source_path,
            file_status,
            policy_target,
        ) =
            query_row.map_err(
                |error| {
                    format!(
                        "Unable to decode Policy List database row: {}",
                        error,
                    )
                }
            )?;


        let policy_target =
            match policy_target
                .trim()
                .to_ascii_lowercase()
                .as_str()
            {
                "screensaver" =>
                    crate::editor_layout::PolicyTarget::Screensaver,

                "wallpaper" =>
                    crate::editor_layout::PolicyTarget::Wallpaper,

                "unassigned" =>
                    crate::editor_layout::PolicyTarget::Unassigned,

                other => {
                    return Err(
                        format!(
                            "Policy '{}' has unsupported policy_target '{}'",
                            policy_name,
                            other,
                        )
                    );
                }
            };


        let resolved_path =
            PathBuf::from(
                source_path
            )
            .join(
                &filename
            );


        display_rows.push(
            crate::editor_layout::PolicyDisplayRow {
                policy_id,

                policy_key:
                    policy_name,

                filename,

                full_path:
                    resolved_path
                        .display()
                        .to_string(),

                accessible:
                    file_status != "missing"
                        && resolved_path.is_file(),

                texture:
                    shader_requires_texture_for_policy_row(
                        &resolved_path
                    ),

                policy_target,

                unassigned:
                    policy_target
                        == crate::editor_layout::PolicyTarget::Unassigned,
            }
        );
    }


    Ok(
        display_rows
    )
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


fn shader_id_for_control_center_path(
    path: &Path,
) -> Result<Option<i64>, String> {

    let filename =
        path.file_name()
            .and_then(
                |name| name.to_str()
            )
            .ok_or_else(
                || {
                    format!(
                        "Shader path has no valid filename: {}",
                        path.display(),
                    )
                }
            )?;


    let parent =
        path.parent()
            .unwrap_or_else(
                || Path::new(".")
            )
            .to_string_lossy()
            .to_string();


    let connection =
        crate::open_database::open()?;


    match connection.query_row(
        "SELECT shader_id
         FROM shaders
         WHERE filename = ?1
           AND source_path = ?2
         ORDER BY shader_id
         LIMIT 1",
        rusqlite::params![
            filename,
            parent,
        ],
        |row| {
            row.get::<_, i64>(
                0
            )
        },
    ) {
        Ok(shader_id) => {
            Ok(
                Some(
                    shader_id
                )
            )
        }

        Err(
            rusqlite::Error::QueryReturnedNoRows
        ) => {
            Ok(
                None
            )
        }

        Err(error) => {
            Err(
                format!(
                    "Unable to query shader ID for '{}': {}",
                    path.display(),
                    error,
                )
            )
        }
    }
}


fn resolve_shader_by_id_for_control_center(
    shader_id: i64,
    preferred_target:
        Option<
            crate::editor_layout::PolicyTarget
        >,
) -> Result<
    Option<(
        PathBuf,
        Option<crate::editor_layout::PolicyTarget>,
    )>,
    String,
> {

    let connection =
        crate::open_database::open()?;


    let preferred_target_name =
        preferred_target
            .map(
                |target| {
                    match target {
                        crate::editor_layout::PolicyTarget::Screensaver =>
                            "screensaver",

                        crate::editor_layout::PolicyTarget::Wallpaper =>
                            "wallpaper",

                        crate::editor_layout::PolicyTarget::Unassigned =>
                            "unassigned",
                    }
                }
            );


    let row =
        connection.query_row(
            "SELECT
                 s.filename,
                 s.source_path,
                 (
                     SELECT p.policy_target
                     FROM shader_policies AS p
                     WHERE p.shader_id = s.shader_id
                     ORDER BY
                         CASE
                             WHEN p.policy_target = ?2 THEN 0
                             ELSE 1
                         END,
                         p.policy_id
                     LIMIT 1
                 )
             FROM shaders AS s
             WHERE s.shader_id = ?1
               AND s.file_status <> 'missing'",
            rusqlite::params![
                shader_id,
                preferred_target_name,
            ],
            |row| {
                Ok(
                    (
                        row.get::<_, String>(
                            0
                        )?,
                        row.get::<_, String>(
                            1
                        )?,
                        row.get::<_, Option<String>>(
                            2
                        )?,
                    )
                )
            },
        );


    let (
        filename,
        source_path,
        target_name,
    ) =
        match row {
            Ok(row) => {
                row
            }

            Err(
                rusqlite::Error::QueryReturnedNoRows
            ) => {
                return Ok(
                    None
                );
            }

            Err(error) => {
                return Err(
                    format!(
                        "Unable to resolve shader_id {}: {}",
                        shader_id,
                        error,
                    )
                );
            }
        };


    let target =
        match target_name
            .as_deref()
        {
            Some("screensaver") => {
                Some(
                    crate::editor_layout::PolicyTarget::Screensaver
                )
            }

            Some("wallpaper") => {
                Some(
                    crate::editor_layout::PolicyTarget::Wallpaper
                )
            }

            Some("unassigned") => {
                Some(
                    crate::editor_layout::PolicyTarget::Unassigned
                )
            }

            _ => {
                None
            }
        };


    Ok(
        Some(
            (
                PathBuf::from(
                    source_path
                )
                .join(
                    filename
                ),
                target,
            )
        )
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


fn load_control_center_state() -> ControlCenterState {

    let state_path =
        crate::locate_paths::state_path();


    if let Ok(text) =
        std::fs::read_to_string(
            &state_path
        )
    {
        match serde_json::from_str::<ControlCenterState>(
            &text
        ) {
            Ok(state) => {
                return state;
            }

            Err(error) => {
                log_warning(
                    &format!(
                        "[EDIT_SHADER] Ignoring invalid Control Center state at {}: {}",
                        state_path.display(),
                        error,
                    )
                );

                return ControlCenterState::default();
            }
        }
    }


    let legacy_path =
        crate::locate_paths::legacy_recent_shader_history_path();


    let mut state =
        ControlCenterState::default();


    if let Ok(text) =
        std::fs::read_to_string(
            &legacy_path
        )
    {
        if let Ok(stored_paths) =
            serde_json::from_str::<Vec<String>>(
                &text
            )
        {
            state.recent_shaders =
                stored_paths;


            if save_control_center_state(
                &state
            )
            .is_ok()
            {
                let _ =
                    std::fs::remove_file(
                        &legacy_path
                    );

                log_information(
                    "[EDIT_SHADER] Migrated recent-shaders.json to state.json"
                );
            }
        }
    }


    state
}


fn save_control_center_state(
    state: &ControlCenterState,
) -> Result<(), String> {

    let state_path =
        crate::locate_paths::state_path();


    if let Some(parent) =
        state_path.parent()
    {
        std::fs::create_dir_all(
            parent
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create Control Center state folder {}: {}",
                    parent.display(),
                    error,
                )
            }
        )?;
    }


    let serialized =
        serde_json::to_string_pretty(
            state
        )
        .map_err(
            |error| {
                format!(
                    "Unable to serialize Control Center state: {}",
                    error,
                )
            }
        )?;


    std::fs::write(
        &state_path,
        serialized,
    )
    .map_err(
        |error| {
            format!(
                "Unable to write Control Center state {}: {}",
                state_path.display(),
                error,
            )
        }
    )
}


fn load_recent_shader_paths() -> Vec<PathBuf> {

    let mut state =
        load_control_center_state();


    let mut recent_paths =
        Vec::new();


    for stored_path in
        state.recent_shaders
            .drain(..)
    {
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

    let mut state =
        load_control_center_state();


    state.recent_shaders =
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


    save_control_center_state(
        &state
    )
}


fn policy_target_state_name(
    target: crate::editor_layout::PolicyTarget,
) -> &'static str {

    match target {
        crate::editor_layout::PolicyTarget::Screensaver =>
            "screensaver",

        crate::editor_layout::PolicyTarget::Wallpaper =>
            "wallpaper",

        crate::editor_layout::PolicyTarget::Unassigned =>
            "unassigned",
    }
}


fn restored_policy_row(
    state: &PersistentPolicyListState,
    policy_rows: &[crate::editor_layout::PolicyDisplayRow],
) -> Option<crate::editor_layout::PolicyRowReference> {

    let identity =
        state.last_edited_policy
            .as_ref()?;


    let target =
        match identity
            .policy_target
            .as_str()
        {
            "screensaver" =>
                crate::editor_layout::PolicyTarget::Screensaver,

            "wallpaper" =>
                crate::editor_layout::PolicyTarget::Wallpaper,

            "unassigned" =>
                crate::editor_layout::PolicyTarget::Unassigned,

            _ =>
                return None,
        };


    policy_rows
        .iter()
        .find(
            |row| {
                identity.policy_id
                    .is_some_and(
                        |policy_id| row.policy_id == policy_id
                    )
            }
        )
        .or_else(
            || {
                policy_rows
                    .iter()
                    .find(
                        |row| {
                            identity.policy_id.is_none()
                                && row.policy_target == target
                                && row.policy_key.eq_ignore_ascii_case(
                                    &identity.policy_key
                                )
                                && (identity.source_path.is_empty()
                                    || row.full_path == identity.source_path)
                        }
                    )
            }
        )
        .map(
            |row| {
                crate::editor_layout::PolicyRowReference {
                    policy_id:
                        row.policy_id,

                    policy_key:
                        row.policy_key.clone(),

                    filename:
                        row.filename.clone(),

                    full_path:
                        row.full_path.clone(),

                    policy_target:
                        row.policy_target,

                    unassigned:
                        row.unassigned,
                }
            }
        )
}


fn restore_policy_list_state(
    edit_window: &mut EditWindowOverlay,
    policy_rows: &[crate::editor_layout::PolicyDisplayRow],
) {
    let state =
        load_control_center_state();


    edit_window.restore_policy_list_state(
        &state.policy_list.sort_column,
        state.policy_list.sort_ascending,
        restored_policy_row(
            &state.policy_list,
            policy_rows,
        ),
        state.window.x,
        state.window.y,
    );
}


fn save_policy_list_state_if_changed(
    edit_window: &EditWindowOverlay,
    last_saved:
        &mut crate::editor_layout::PolicyListStateSnapshot,
) {
    let snapshot =
        edit_window
            .policy_list_state_snapshot();


    if &snapshot
        == last_saved
    {
        return;
    }


    let mut state =
        load_control_center_state();


    state.policy_list.sort_column =
        snapshot.sort_column.clone();

    state.policy_list.sort_ascending =
        snapshot.sort_ascending;

    state.policy_list.last_edited_policy =
        snapshot.selected_policy_row
            .as_ref()
            .map(
                |row| {
                    PersistentPolicyIdentity {
                        policy_id:
                            Some(row.policy_id),

                        policy_key:
                            row.policy_key.clone(),

                        policy_target:
                            policy_target_state_name(
                                row.policy_target
                            )
                            .to_string(),

                        source_path:
                            row.full_path.clone(),
                    }
                }
            );

    state.window.x =
        snapshot.window_x;

    state.window.y =
        snapshot.window_y;


    match save_control_center_state(
        &state
    ) {
        Ok(()) => {
            *last_saved =
                snapshot;
        }

        Err(error) => {
            log_warning(
                &format!(
                    "[EDIT_SHADER] Unable to save Policy List state: {}",
                    error,
                )
            );
        }
    }
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
                        crate::palettes::PaletteColor::parse_hex(
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
            crate::palettes::PaletteColor
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

        crate::palettes::PaletteColor::parse_hex(
            &normalized
        )
        .map(
            Some
        )
    }


    fn build_mode(
        display: &str,
        interval_seconds: u64,
        single_policy_id: Option<i64>,
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
                let policy_id =
                    single_policy_id
                        .filter(
                            |policy_id| {
                                *policy_id > 0
                            }
                        )
                        .ok_or_else(
                            || {
                                "Single display mode requires a shader policy selection."
                                    .to_string()
                            }
                        )?;

                Ok(
                    format!(
                        "single:{}",
                        policy_id,
                    )
                )
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
                    control.screensaver_single_policy_id,
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
                    control.wallpaper_single_policy_id,
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


fn bloom_selection_from_mode(
    mode: crate::render_bloom::BloomMode,
) -> crate::editor_layout::BloomSelection {
    match mode {
        crate::render_bloom::BloomMode::Off => {
            crate::editor_layout::BloomSelection::Off
        }

        crate::render_bloom::BloomMode::Highlight => {
            crate::editor_layout::BloomSelection::Highlight
        }

        crate::render_bloom::BloomMode::Audio => {
            crate::editor_layout::BloomSelection::Audio
        }
    }
}


fn bloom_mode_from_selection(
    selection: crate::editor_layout::BloomSelection,
) -> crate::render_bloom::BloomMode {
    match selection {
        crate::editor_layout::BloomSelection::Off => {
            crate::render_bloom::BloomMode::Off
        }

        crate::editor_layout::BloomSelection::Highlight => {
            crate::render_bloom::BloomMode::Highlight
        }

        crate::editor_layout::BloomSelection::Audio => {
            crate::render_bloom::BloomMode::Audio
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

