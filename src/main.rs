mod initialize_user_files;
mod startup_checks;
mod load_config;
mod singleton;
mod logger;
mod parse_mode;
mod parse_interval;
mod parse_duration;
mod parse_arguments;
mod parse_subtitle_placement;
mod parse_texture_specification;

mod define_constants;
mod locate_paths;
mod delete_cache;
mod manage_cache;

mod query_session;
mod session_backend;
mod audio_backend;

mod manage_configuration;
mod manage_shader;
mod manage_textures;
mod manage_policies;
mod classify_shader;
mod isf_types;
mod parse_isf;
mod preprocess_isf;
mod apply_shader_inputs;
mod preprocess_shader;
mod load_shader;
mod compile_shader;
mod render_frame;
mod splash_screen;
mod reject_shader;
mod generate_bricks;
mod generate_cellular;
mod generate_clouds;
mod generate_facets;
mod generate_hexagons;
mod generate_marble;
mod generate_mesh;
mod generate_noise;
mod generate_radial;
mod generate_textures;
mod preview_texture;
mod preview_shader;
mod edit_shader;
mod editor_layout;
mod editor_theme;
mod preview_shader_directory;
mod palettes;
mod display_texture;
mod display_message;
mod construct_text_overlay;
mod display_overlay;
mod tray_icon;

mod define_operation;
mod define_wallpaper;
mod configure_wallpaper;
mod control_wallpaper;
mod locate_wallpaper;
mod manage_wallpaper;
mod manage_wallpaper_runtime;
mod notify_wallpaper;
mod render_wallpaper;
mod wayland_wallpaper;
mod wallpaper_backend;
mod x11_connection;
mod x11_wallpaper;
mod glx_context;
mod fps_monitor;

mod postprocess_shader;
mod render_passthrough;
mod render_fxaa;
mod render_dithering;
mod render_bloom;
mod select_render_precision;

use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::time::Duration;
use std::io::{self, Write};



fn confirm_policy_replacement(
    shader: &str,
    target: crate::manage_policies::PolicyTarget,
) -> Result<bool, String> {

    loop {
        print!(
            "Shader '{}' already has an policy in [{}] -- delete it? [Y/n] ",
            shader,
            target.table_name(),
        );

        io::stdout()
            .flush()
            .map_err(
                |error| error.to_string()
            )?;

        let mut response =
            String::new();

        io::stdin()
            .read_line(
                &mut response
            )
            .map_err(
                |error| error.to_string()
            )?;

        match response
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "" | "y" | "yes" => {
                return Ok(true);
            }

            "n" | "no" => {
                return Ok(false);
            }

            _ => {
                println!(
                    "Please answer Y or n."
                );
            }
        }
    }
}


fn main() {

let config_dir = match initialize_user_files::initialize() {
    Ok(path) => path,

    Err(error) => {
        eprintln!(
            "Screenshaver could not initialize its user files: {error}"
        );
        std::process::exit(1);
    }
};

println!(
    "Screenshaver configuration directory: {}",
    config_dir.display()
);

let command =
    match crate::parse_arguments::parse() {

        Ok(command) => command,

        Err(error) => {

            crate::parse_arguments::print_error(
                &error
            );

            return;
        }
    };


let runtime_logfile =
    match &command {

        crate::parse_arguments::Command::Run
        | crate::parse_arguments::Command::Start
        | crate::parse_arguments::Command::PreviewTexture { .. }
        | crate::parse_arguments::Command::PreviewShader { .. }
        | crate::parse_arguments::Command::Control { .. }
        | crate::parse_arguments::Command::EditShader { .. }
        | crate::parse_arguments::Command::DeleteCache => {

            let logfile =
                crate::locate_paths::runtime_log_path();


            crate::logger::reset_log(
                &logfile
            );


            Some(logfile)
        }

        _ => {
            None
        }
    };


match command {

    crate::parse_arguments::Command::Run
    | crate::parse_arguments::Command::Start => {}


    crate::parse_arguments::Command::Stop => {

        match crate::singleton::stop() {

            Ok(
                crate::singleton::StopOutcome::StopRequested {
                    pid,
                }
            ) => {

                println!(
                    "Screenshaver stop requested for process {}.",
                    pid
                );
            }

            Ok(
                crate::singleton::StopOutcome::NotRunning
            ) => {

                println!(
                    "Screenshaver is not running."
                );
            }

            Err(error) => {

                eprintln!(
                    "[MAIN] STOP ERROR: {}",
                    error
                );
            }
        }


        return;
    }


    crate::parse_arguments::Command::AddPolicy {
        target,
        shader,
        properties,
    } => {

        let cfg_path =
            crate::locate_paths::config_path();

        let exists =
            match crate::manage_policies::policy_exists(
                &cfg_path,
                target,
                &shader,
            ) {
                Ok(exists) => exists,

                Err(error) => {
                    eprintln!(
                        "{}",
                        error
                    );

                    return;
                }
            };

        if exists {
            let replace =
                match confirm_policy_replacement(
                    &shader,
                    target,
                ) {
                    Ok(replace) => replace,

                    Err(error) => {
                        eprintln!(
                            "Unable to read confirmation: {}",
                            error
                        );

                        return;
                    }
                };

            if !replace {
                println!(
                    "Policy addition cancelled."
                );

                return;
            }

            match crate::manage_policies::replace_policy(
                &cfg_path,
                target,
                &shader,
                properties,
            ) {
                Ok(()) => {
                    println!(
                        "Replaced {} policy for {}.",
                        target.name(),
                        shader,
                    );
                }

                Err(error) => {
                    eprintln!(
                        "{}",
                        error
                    );
                }
            }
        } else {
            match crate::manage_policies::add_policy(
                &cfg_path,
                target,
                &shader,
                properties,
            ) {
                Ok(()) => {
                    println!(
                        "Added {} policy for {}.",
                        target.name(),
                        shader,
                    );
                }

                Err(error) => {
                    eprintln!(
                        "{}",
                        error
                    );
                }
            }
        }

        return;
    }


    crate::parse_arguments::Command::DeletePolicy {
        target,
        shader,
    } => {

        let cfg_path =
            crate::locate_paths::config_path();

        match crate::manage_policies::delete_policy(
            &cfg_path,
            target,
            &shader,
        ) {
            Ok(()) => {
                println!(
                    "Deleted {} policy for {}.",
                    target.name(),
                    shader,
                );
            }

            Err(error) => {
                eprintln!(
                    "{}",
                    error
                );
            }
        }

        return;
    }


    crate::parse_arguments::Command::ListPolicies {
        target,
    } => {

        let cfg_path =
            crate::locate_paths::config_path();

        if let Err(error) =
            crate::manage_policies::list_policies(
                &cfg_path,
                target,
            )
        {
            eprintln!(
                "{}",
                error
            );
        }

        return;
    }


    crate::parse_arguments::Command::Help => {

        crate::parse_arguments::print_help();

        return;
    }


    crate::parse_arguments::Command::Version => {

        crate::parse_arguments::print_version();

        return;
    }


    crate::parse_arguments::Command::Reserved {
        option,
    } => {

        crate::parse_arguments::print_reserved_option(
            &option
        );

        return;
    }

    crate::parse_arguments::Command::PreviewTexture {
        texture,
        palette,
    } => {

        crate::preview_texture::run(
            texture,
            palette,
        );

        return;
    }

crate::parse_arguments::Command::PreviewShader {
    shader_name,
    shader_texture,
    shader_palette,
    interval_seconds,
    fps,
    animation_speed,
    bloom,
    bloom_intensity,
    bloom_threshold,
} => {

    match crate::preview_shader::run(
        shader_name,
        shader_texture,
        shader_palette,
        interval_seconds,
        fps,
        animation_speed,
        bloom,
        bloom_intensity,
        bloom_threshold,
    ) {

        Ok(()) => {}

        Err(error) => {

            eprintln!(
                "[SHADER PREVIEW] {}",
                error
            );


            if let Some(logfile) =
                runtime_logfile.as_ref()
            {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[PREVIEW_SHADER] {}",
                        error,
                    ),
                );
            }
        }
    }


    return;
}


crate::parse_arguments::Command::Control {
    shader_name,
} => {

    match crate::edit_shader::run(
        shader_name
    ) {

        Ok(()) => {}

        Err(error) => {

            eprintln!(
                "[CONTROL CENTER] {}",
                error
            );


            if let Some(logfile) =
                runtime_logfile.as_ref()
            {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[CONTROL_CENTER] {}",
                        error,
                    ),
                );
            }
        }
    }


    return;
}


crate::parse_arguments::Command::EditShader {
    shader_name,
} => {

    match crate::edit_shader::run(
        shader_name
    ) {

        Ok(()) => {}

        Err(error) => {

            eprintln!(
                "[SHADER EDITOR] {}",
                error
            );


            if let Some(logfile) =
                runtime_logfile.as_ref()
            {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[EDIT_SHADER] {}",
                        error,
                    ),
                );
            }
        }
    }


    return;
}


crate::parse_arguments::Command::DeleteCache => {

    match crate::delete_cache::run() {

        Ok(()) => {}

        Err(error) => {

            eprintln!(
                "[CACHE] {}",
                error
            );


            if let Some(logfile) =
                runtime_logfile.as_ref()
            {
                crate::logger::error(
                    logfile,
                    &format!(
                        "[CACHE] {}",
                        error,
                    ),
                );
            }
        }
    }


    return;
}


crate::parse_arguments::Command::ListTextures => {

    println!(
        "Available texture families:"
    );

    println!(
        "    marble"
    );

    println!(
        "    clouds"
    );

    println!(
        "    cells"
    );

    println!(
        "    mesh"
    );

    println!(
        "    radial"
    );

    println!(
        "    noise"
    ); 

    println!(
        "    bricks"
    );

    println!(
        "    hexagons"
    ); 

    return;
}


}

     let identity =
        crate::startup_checks::current_user_identity();

    if identity.is_root() {

        let security_logfile =
            crate::locate_paths::runtime_log_path();


        let security_message =
            format!(
                "[SECURITY] Refusing root execution: \
                 real_uid={} effective_uid={}",
                identity.real_uid,
                identity.effective_uid,
            );


        eprintln!(
            "{}",
            security_message
        );


        crate::logger::ensure_log_exists(
            &security_logfile
        );


        crate::logger::error(
            &security_logfile,
            &security_message,
        );


        crate::logger::information(
            &security_logfile,
            "[SECURITY] Screenshaver terminated after refusing root execution.",
        );


        return;
    }

    println!(
        "[MAIN] Loading configuration..."
    );


    let cfg_path =
        crate::locate_paths::config_path();


    let result =
        match crate::load_config::load_config(
            &cfg_path
        ) {

            Ok(result) => result,

            Err(error) => {

                eprintln!(
                    "[MAIN] CONFIG ERROR: {}",
                    error
                );


                if let Some(logfile) =
                    runtime_logfile.as_ref()
                {
                    crate::logger::error(
                        logfile,
                        &format!(
                            "[CONFIG] Unable to load configuration: {}",
                            error,
                        ),
                    );
                }


                return;
            }
        };


    let mut cfg =
        result.config;


    let logfile =
        runtime_logfile
            .as_ref()
            .expect(
                "Runtime command did not initialize the log path"
            )
            .clone();


    crate::logger::set_enabled(
        cfg.debug_log
    );


    crate::logger::set_log_level(
        cfg.log_level
    );


    crate::logger::information(
        &logfile,
        "[MAIN] Screenshaver runtime started",
    );


    if cfg.debug_log {

        crate::logger::debug(
            &logfile,
            "[MAIN] === CONFIG DUMP ===",
        );


        for line in &result.diagnostics {

            crate::logger::debug(
                &logfile,
                line,
            );
        }


        crate::logger::debug(
            &logfile,
            "[MAIN] === CONFIG END ===",
        );
    }


    let _singleton =
        match crate::singleton::acquire() {

            Ok(singleton) => {
                singleton
            }

            Err(
                crate::singleton::SingletonError::AlreadyRunning
            ) => {

                println!(
                    "Screenshaver is already running."
                );

                return;
            }

            Err(error) => {

                eprintln!(
                    "[MAIN] SINGLETON ERROR: {}",
                    error
                );


                if let Some(logfile) =
                    runtime_logfile.as_ref()
                {
                    crate::logger::error(
                        logfile,
                        &format!(
                            "[MAIN] Singleton acquisition failed: {}",
                            error,
                        ),
                    );
                }


                return;
            }
        };


    if let Err(error) =
        crate::manage_cache::delete_stale_cache_entries()
    {
        crate::logger::warning(
            &logfile,
            &format!(
                "[CACHE] Garbage collection was skipped: {}",
                error,
            ),
        );
    }


    let (
        tray_command_sender,
        tray_command_receiver,
    ) =
        std::sync::mpsc::channel::<
            crate::tray_icon::TrayCommand
        >();

    let tray_status =
        crate::tray_icon::TrayStatusControl::new(
            cfg.wallpaper_enabled
        );


    let _tray_handle =
        match crate::tray_icon::start(
            tray_command_sender,
            crate::tray_icon::TrayStatus {
                screensaver_enabled:
                    cfg.screensaver_enabled,
                wallpaper:
                    tray_status.clone(),
            },
        ) {

            Ok(handle) => {

                println!(
                    "[TRAY] System tray icon registered successfully."
                );


                if let Some(logfile) =
                    runtime_logfile.as_ref()
                {
                    crate::logger::information(
                        logfile,
                        "[TRAY] System tray icon registered successfully",
                    );
                }


                Some(handle)
            }

            Err(error) => {

                eprintln!(
                    "[TRAY] System tray icon unavailable: {:?}",
                    error
                );


                if let Some(logfile) =
                    runtime_logfile.as_ref()
                {
                    crate::logger::warning(
                        logfile,
                        &format!(
                            "[TRAY] System tray icon unavailable: {:?}",
                            error,
                        ),
                    );
                }


                None
            }
        };

    println!(
        "[MAIN] Parsing shader mode..."
    );


    let parsed_mode =
        crate::parse_mode::parse_mode(
            &cfg.mode
        );


    println!(
        "[MAIN] Mode = {:?}",
        parsed_mode.mode
    );


    println!(
        "[MAIN] Argument = {}",
        parsed_mode.argument
    );


    if cfg.debug_log {

        crate::logger::debug(
            &logfile,
            "[MAIN] === MODE PARSE ===",
        );


        for line in &parsed_mode.diagnostics {

            crate::logger::debug(
                &logfile,
                line,
            );
        }
    }


    println!(
        "[MAIN] Parsing shader interval..."
    );


    let parsed_interval =
        match cfg.mode
            .split(':')
            .next()
            .unwrap_or("single")
        {

            "single" => {

                if cfg.debug_log {

                    crate::logger::debug(
                        &logfile,
                        "[MAIN] === INTERVAL SKIPPED (SINGLE MODE) ===",
                    );
                }


                crate::parse_interval::ParsedInterval {
                    seconds: 0,

                    diagnostics: vec![
                        "skipped".to_string()
                    ],
                }
            }


            "random" | "ordered" => {

                let interval_source =
                    cfg.mode
                        .split(':')
                        .nth(1)
                        .unwrap_or("60");


                let result =
                    crate::parse_interval::parse_interval(
                        interval_source
                    );


                if cfg.debug_log {

                    crate::logger::debug(
                        &logfile,
                        "[MAIN] === INTERVAL PARSE ===",
                    );


                    for line in &result.diagnostics {

                        crate::logger::debug(
                            &logfile,
                            line,
                        );
                    }
                }


                result
            }


            _ => {

                crate::parse_interval::ParsedInterval {
                    seconds: 0,

                    diagnostics: vec![
                        "invalid mode".to_string()
                    ],
                }
            }
        };


    println!(
        "[MAIN] Parsing idle timeout..."
    );


    let parsed_idle =
        crate::parse_duration::parse_duration(
            &cfg.idle_timeout
        );


    println!(
        "[MAIN] Idle timeout = {:?}",
        parsed_idle.duration
    );


    if cfg.debug_log {

        crate::logger::debug(
            &logfile,
            "[MAIN] === IDLE PARSE ===",
        );


        for line in &parsed_idle.diagnostics {

            crate::logger::debug(
                &logfile,
                line,
            );
        }
    }


    let session =
        match crate::query_session::SessionQuery::new(
            parsed_idle.duration
        ) {

            Ok(session) => session,

            Err(error) => {

                eprintln!(
                    "[MAIN] SESSION ERROR: {}",
                    error
                );


                crate::logger::error(
                    &logfile,
                    &format!(
                        "[SESSION] Unable to initialize session query: {}",
                        error,
                    ),
                );


                return;
            }
        };


    println!(
        "[MAIN] Session backend = {}",
        session.backend_name()
    );


    crate::logger::information(
        &logfile,
        &format!(
            "[SESSION] Session backend: {}",
            session.backend_name(),
        ),
    );


    let sdl =
        match sdl2::init() {

            Ok(sdl) => sdl,

            Err(error) => {

                eprintln!(
                    "[MAIN] SDL initialization failed: {}",
                    error
                );


                crate::logger::error(
                    &logfile,
                    &format!(
                        "[SDL] Initialization failed: {}",
                        error,
                    ),
                );


                return;
            }
        };


    if cfg.show_splash {

        println!(
            "[MAIN] Displaying splash screen..."
        );


        crate::logger::information(
            &logfile,
            "[SPLASH] Displaying splash screen",
        );


        match crate::splash_screen::show_splash(
            &sdl
        ) {

            Ok(()) => {

                crate::logger::information(
                    &logfile,
                    "[SPLASH] Splash screen complete",
                );
            }

            Err(error) => {

                eprintln!(
                    "[MAIN] Splash screen failed: {}",
                    error
                );


                crate::logger::warning(
                    &logfile,
                    &format!(
                        "[SPLASH] Splash screen failed: {}",
                        error,
                    ),
                );
            }
        }
    }

    let running =
        Arc::new(
            AtomicBool::new(true)
        );


    let signal_running =
        running.clone();


    ctrlc::set_handler(
        move || {

            signal_running.store(
                false,
                Ordering::SeqCst,
            );
        }
    )
    .expect(
        "Ctrl-C handler failed"
    );


    let wallpaper_runtime =
        crate::define_wallpaper::WallpaperRuntime {
            monitor_mode:
                cfg.wallpaper.monitor_mode,
            notifications:
                cfg.wallpaper.notifications,
            texture_policy:
                cfg.wallpaper_texture_policy,
            fps_policy:
                cfg.wallpaper_fps_policy,
            animation_speed_policy:
                cfg.wallpaper_speed_policy,
            postprocess_policy:
                cfg.wallpaper_postprocess_policy,
            tray_status:
                tray_status.clone(),
        };


    let mut wallpaper_manager =
        crate::manage_wallpaper_runtime::WallpaperRuntimeManager::start(
            cfg.wallpaper_enabled,
            cfg.wallpaper_mode.clone(),
            wallpaper_runtime,
            logfile.clone(),
            Arc::clone(
                &running
            ),
        );


    let wallpaper_control =
        wallpaper_manager.control();


    if cfg.debug_log {

        crate::logger::debug(
            &logfile,
            "[MAIN] === ENTERING SESSION LOOP ===",
        );
    }


    println!(
        "[MAIN] Entering session loop..."
    );


    let mut restart_requested = false;


    let mut tray_channel_connected =
        true;


    while running.load(Ordering::SeqCst) {

        if tray_channel_connected {

            match tray_command_receiver.try_recv() {

            Ok(crate::tray_icon::TrayCommand::Stop) => {

                println!(
                    "[TRAY] Stop requested."
                );


                crate::logger::information(
                    &logfile,
                    "[TRAY] Stop requested",
                );


                running.store(
                    false,
                    Ordering::SeqCst,
                );

                break;
            }


            Ok(crate::tray_icon::TrayCommand::Edit) => {

                println!(
                    "[TRAY] Control Center requested."
                );

                crate::logger::information(
                    &logfile,
                    "[TRAY] Control Center requested",
                );

                let active_wallpaper =
                    tray_status.active_wallpaper();

                if active_wallpaper.is_some() {
                    wallpaper_control.request_pause_after_first_frame(
                        running.as_ref()
                    );
                }

                let edit_result =
                    match active_wallpaper {
                        Some(active_wallpaper) => {
                            crate::edit_shader::run_wallpaper_only(
                                active_wallpaper.path
                            )
                        }

                        None => {
                            crate::edit_shader::run(
                                None
                            )
                        }
                    };

                if edit_result.is_ok() {
                    let config_path =
                        crate::locate_paths::config_path();

                    match crate::load_config::load_config(
                        &config_path
                    ) {
                        Ok(config_result) => {
                            for diagnostic in
                                &config_result.diagnostics
                            {
                                crate::logger::warning(
                                    &logfile,
                                    &format!(
                                        "[TRAY] Configuration reload diagnostic: {}",
                                        diagnostic,
                                    ),
                                );
                            }

                            cfg = config_result.config;

                            wallpaper_control.request_policy_reload(
                                crate::manage_wallpaper_runtime::WallpaperPolicyReload::from_config(
                                    &cfg
                                )
                            );

                            crate::logger::information(
                                &logfile,
                                "[TRAY] Reloaded configuration after Control Center closed",
                            );
                        }

                        Err(error) => {
                            crate::logger::error(
                                &logfile,
                                &format!(
                                    "[TRAY] Unable to reload configuration after editing: {}",
                                    error,
                                ),
                            );
                        }
                    }
                }

                if tray_status.active_wallpaper().is_some() {
                    wallpaper_control.resume_and_wait_for_frame(
                        running.as_ref()
                    );
                }

                match edit_result {
                    Ok(()) => {
                        crate::logger::information(
                            &logfile,
                            "[TRAY] Control Center closed",
                        );
                    }

                    Err(error) => {
                        eprintln!(
                            "[TRAY] Unable to open Control Center: {}",
                            error
                        );

                        crate::logger::error(
                            &logfile,
                            &format!(
                                "[TRAY] Unable to open Control Center: {}",
                                error,
                            ),
                        );
                    }
                }
            }


            Ok(crate::tray_icon::TrayCommand::Restart) => {

                println!(
                    "[TRAY] Restart requested."
                );


                crate::logger::information(
                    &logfile,
                    "[TRAY] Restart requested",
                );


                restart_requested = true;

                running.store(
                    false,
                    Ordering::SeqCst,
                );

                break;
            }


            Err(std::sync::mpsc::TryRecvError::Empty) => {}


            Err(std::sync::mpsc::TryRecvError::Disconnected) => {

                eprintln!(
                    "[TRAY] Command channel disconnected."
                );


                crate::logger::warning(
                    &logfile,
                    "[TRAY] Command channel disconnected",
                );


                tray_channel_connected =
                    false;
            }
        }
        }

        if !cfg.screensaver_enabled {

            std::thread::sleep(
                Duration::from_millis(50)
            );

            continue;
        }


        let session_state =
            match session.poll_state() {

                Ok(state) => state,

                Err(error) => {

                    eprintln!(
                        "[MAIN] SESSION QUERY ERROR: {}",
                        error
                    );


                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[SESSION] Session query failed: {}",
                            error,
                        ),
                    );


                    break;
                }
            };


        match session_state {

            crate::query_session::SessionState::Idle => {

                crate::logger::information(
                    &logfile,
                    "[SESSION] Session idle: engaging renderer",
                );

                println!(
                    "[MAIN] Session idle: engaging renderer"
                );

                let configured_shader_mode =
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

                let mut next_shader_mode =
                    configured_shader_mode.clone();

                let mut next_shader_interval =
                    parsed_interval.seconds;

                let mut next_initial_shader:
                    Option<String> =
                    None;

                'screensaver_session: loop {
                    let shader_manager =
                        if let Some(initial_shader) =
                            next_initial_shader.take()
                        {
                            crate::manage_shader::ShaderManager::new_with_initial_shader(
                                next_shader_mode.clone(),
                                initial_shader,
                            )
                        } else {
                            crate::manage_shader::ShaderManager::new(
                                next_shader_mode.clone()
                            )
                        };

                    let mut renderer =
                        match crate::render_frame::FrameRenderer::new(
                            &sdl,
                            shader_manager,
                            next_shader_interval,
                            cfg.screensaver_speed_policy.clone(),
                            cfg.global_rendered_fps,
                            cfg.screensaver_fps_policy_entries.clone(),
                            cfg.texture_policy.clone(),
                            cfg.screensaver_postprocess_policy.clone(),
                            cfg.subtitles,
                            cfg.subtitle_placement,
                        ) {
                            Ok(renderer) => renderer,

                            Err(error) => {
                                eprintln!(
                                    "[MAIN] Renderer initialization failed: {}",
                                    error
                                );

                                crate::logger::error(
                                    &logfile,
                                    &format!(
                                        "[RENDER] Renderer initialization failed: {}",
                                        error,
                                    ),
                                );

                                wallpaper_control.resume_and_wait_for_frame(
                                    running.as_ref()
                                );

                                break 'screensaver_session;
                            }
                        };

                    crate::logger::information(
                        &logfile,
                        "[RENDER] Renderer started",
                    );

                    let renderer_outcome =
                        renderer.run(
                            running.as_ref(),
                            &wallpaper_control,
                        );

                    drop(renderer);

                    match renderer_outcome {
                        crate::render_frame::ScreensaverRunOutcome::Exit => {
                            if running.load(Ordering::SeqCst) {
                                crate::logger::information(
                                    &logfile,
                                    "[SESSION] User input: disengaging renderer",
                                );

                                println!(
                                    "[MAIN] User input: disengaging renderer"
                                );
                            }

                            break 'screensaver_session;
                        }

                        crate::render_frame::ScreensaverRunOutcome::EditCurrentShader(
                            shader_path
                        ) => {
                            crate::logger::information(
                                &logfile,
                                &format!(
                                    "[SESSION] Editing active screensaver shader: {}",
                                    shader_path.display(),
                                ),
                            );

                            let edit_result =
                                crate::edit_shader::run_screensaver_only(
                                    shader_path.clone()
                                );

                            if let Err(error) = &edit_result {
                                eprintln!(
                                    "[SCREENSAVER EDIT] {}",
                                    error
                                );

                                crate::logger::error(
                                    &logfile,
                                    &format!(
                                        "[SCREENSAVER EDIT] {}",
                                        error,
                                    ),
                                );
                            }

                            let config_path =
                                crate::locate_paths::config_path();

                            match crate::load_config::load_config(
                                &config_path
                            ) {
                                Ok(config_result) => {
                                    for diagnostic in
                                        &config_result.diagnostics
                                    {
                                        crate::logger::warning(
                                            &logfile,
                                            &format!(
                                                "[SCREENSAVER EDIT] Configuration reload diagnostic: {}",
                                                diagnostic,
                                            ),
                                        );
                                    }

                                    cfg = config_result.config;
                                }

                                Err(error) => {
                                    crate::logger::error(
                                        &logfile,
                                        &format!(
                                            "[SCREENSAVER EDIT] Unable to reload configuration: {}",
                                            error,
                                        ),
                                    );
                                }
                            }

                            let Some(shader_name) =
                                shader_path
                                    .file_name()
                                    .and_then(
                                        |name| name.to_str()
                                    )
                                    .map(
                                        str::to_string
                                    )
                            else {
                                crate::logger::error(
                                    &logfile,
                                    "[SCREENSAVER EDIT] Active shader path has no valid filename",
                                );

                                wallpaper_control.resume_and_wait_for_frame(
                                    running.as_ref()
                                );

                                break 'screensaver_session;
                            };

                            next_shader_mode =
                                configured_shader_mode.clone();

                            next_shader_interval =
                                parsed_interval.seconds;

                            next_initial_shader =
                                Some(shader_name);

                            if edit_result.is_ok() {
                                crate::logger::information(
                                    &logfile,
                                    "[SCREENSAVER EDIT] Resuming edited screensaver shader",
                                );
                            } else {
                                crate::logger::warning(
                                    &logfile,
                                    "[SCREENSAVER EDIT] Editor failed; resuming the previous shader with the best available configuration",
                                );
                            }
                        }
                    }
                }

                if running.load(Ordering::SeqCst) {
                    if cfg.debug_log {
                        crate::logger::debug(
                            &logfile,
                            "[MAIN] Returning to session loop",
                        );
                    }

                    println!(
                        "[MAIN] Returning to session loop..."
                    );

                    continue;
                } else {
                    break;
                }
            }

            crate::query_session::SessionState::Active => {}
        }


        std::thread::sleep(
            Duration::from_millis(50)
        );
    }


    wallpaper_manager.stop_and_join();


    println!(
        "[MAIN] Pipeline complete."
    );


    crate::logger::information(
        &logfile,
        "[MAIN] Pipeline complete",
    );


    if restart_requested {

        drop(_tray_handle);
        drop(_singleton);


        match std::env::current_exe() {

            Ok(executable) => {

                match std::process::Command::new(
                    executable
                )
                .spawn()
                {

                    Ok(_) => {

                        println!(
                            "[TRAY] Screenshaver restart launched."
                        );


                        crate::logger::information(
                            &logfile,
                            "[TRAY] Screenshaver restart launched",
                        );
                    }


                    Err(error) => {

                        eprintln!(
                            "[TRAY] Unable to restart Screenshaver: {}",
                            error
                        );


                        crate::logger::error(
                            &logfile,
                            &format!(
                                "[TRAY] Unable to restart Screenshaver: {}",
                                error,
                            ),
                        );
                    }
                }
            }


            Err(error) => {

                eprintln!(
                    "[TRAY] Unable to locate Screenshaver executable: {}",
                    error
                );


                crate::logger::error(
                    &logfile,
                    &format!(
                        "[TRAY] Unable to locate Screenshaver executable: {}",
                        error,
                    ),
                );
            }
        }
    }
}

