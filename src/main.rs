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

mod query_session;
mod session_backend;
mod audio_backend;
mod analyze_audio;

mod manage_configuration;
mod manage_shader;
mod manage_screen_lock;
mod manage_screen_lock_gnome;
mod manage_screen_lock_xfce;
mod manage_runtime_xfce;
mod manage_gnome_extension;
mod present_screen_lock_gnome;
mod present_screen_lock_xfce;
mod lock_screen_widget;
mod define_lock_screen_widget;
mod construct_lock_screen_kde;
mod construct_lock_screen_xfce;
mod manage_screen_lock_kde;
mod detect_desktop_environment;
mod manage_textures;
mod manage_policies;
mod classify_shader;
mod isf_types;
mod parse_isf;
mod preprocess_isf;
mod apply_shader_inputs;
mod preprocess_shader;
mod load_shader;
mod load_shader_source;
mod compile_shader;
mod reconcile_shaders;
mod assign_shader_policies;
mod render_frame;
mod splash_screen;
mod generate_bricks;
mod generate_cellular;
mod generate_clouds;
mod generate_eyes;
mod blink_eyes;
mod generate_facets;
mod generate_hexagons;
mod generate_marble;
mod generate_mesh;
mod generate_noise;
mod generate_radial;
mod generate_scales;
mod generate_skulls;
mod generate_textures;
mod preview_texture_thumbnail;
mod edit_shader;
mod editor_layout;
mod nested_tabs;
mod editor_theme;
mod preview_shader_directory;
mod palettes;
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
mod monitor_lock_presentation;

mod postprocess_shader;
mod render_passthrough;
mod render_fxaa;
mod render_dithering;
mod render_bloom;
mod select_render_precision;

mod initialize_database;
mod evaluate_database;
mod open_database;
mod validate_database;
mod migrate_database;
mod hash_shader;

mod qbe_layout;
mod parse_qbe;
mod query_database;

mod create_wayland_lock_context;
mod display_lock_authentication;
mod authenticate_user;

use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::time::Duration;



fn main() {

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


    // Commands that do not require Screenshaver's user environment or
    // database are handled before any configuration/database initialization.
    match &command {

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


        crate::parse_arguments::Command::Help => {

            crate::parse_arguments::print_help();

            return;
        }


        crate::parse_arguments::Command::Version => {

            crate::parse_arguments::print_version();

            return;
        }


        crate::parse_arguments::Command::ConstructLockScreenKde => {

            let desktop_environment =
                crate::detect_desktop_environment::detect();


            println!(
                "Detected desktop environment: {}",
                desktop_environment.name()
            );


            if !desktop_environment.is_kde_plasma() {

                eprintln!(
                    "Screenshaver KDE lock-screen integration was not modified because KDE Plasma is not the detected desktop environment."
                );

                return;
            }


            let config =
                crate::define_lock_screen_widget::LockScreenWidgetConfig::default();


            match crate::manage_screen_lock_kde::install(
                &config
            ) {

                Ok(status) => {

                    println!(
                        "Screenshaver KDE lock screen constructed and installed."
                    );

                    println!(
                        "KDE shell package installed: {}",
                        status.shell_package_installed
                    );

                    println!(
                        "KDE LockScreen.qml installed: {}",
                        status.lockscreen_qml_installed
                    );

                    println!(
                        "Screenshaver selected as KDE lock shell: {}",
                        status.screenshaver_selected
                    );

                    if let Some(active_shell) =
                        status.active_shell_package
                    {

                        println!(
                            "Active KDE shell package: {}",
                            active_shell
                        );
                    }

                    if let Some(previous_shell) =
                        status.previous_shell_package
                    {

                        println!(
                            "Previous KDE shell package preserved: {}",
                            previous_shell
                        );
                    }
                }


                Err(error) => {

                    eprintln!(
                        "Unable to construct and install Screenshaver KDE lock screen: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }


            return;
        }


        crate::parse_arguments::Command::ConstructLockScreenXfce => {

            let desktop_environment =
                crate::detect_desktop_environment::detect();


            println!(
                "Detected desktop environment: {}",
                desktop_environment.name()
            );


            if !desktop_environment.is_xfce() {

                eprintln!(
                    "Screenshaver Xfce lock-screen integration was not modified because Xfce is not the detected desktop environment."
                );

                return;
            }


            match crate::construct_lock_screen_xfce::configure_user() {

                Ok(status) => {

                    println!(
                        "Screenshaver Xfce lock-screen integration configured."
                    );

                    println!(
                        "xfce4-screensaver available: {}",
                        status.xfce_screensaver_available
                    );

                    println!(
                        "xfconf-query available: {}",
                        status.xfconf_query_available
                    );

                    println!(
                        "Trusted Screenshaver presenter installed: {}",
                        status.trusted_presenter_installed
                    );

                    println!(
                        "Screenshaver saver desktop registered: {}",
                        status.saver_desktop_registered
                    );

                    println!(
                        "Xfce saver selection remains native until the resident Screenshaver runtime starts."
                    );

                    println!(
                        "Light Locker autostart disabled: {}",
                        status.light_locker_autostart_disabled
                    );

                    if status.ready_for_runtime() {
                        println!(
                            "Xfce lock-screen integration is ready for runtime use."
                        );
                    } else {
                        eprintln!(
                            "Xfce lock-screen integration is not yet ready for runtime use."
                        );

                        std::process::exit(
                            1
                        );
                    }
                }


                Err(error) => {

                    eprintln!(
                        "Unable to construct and configure Screenshaver Xfce lock-screen integration: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }


            return;
        }


        crate::parse_arguments::Command::Run
        | crate::parse_arguments::Command::Start
        | crate::parse_arguments::Command::Control { .. } => {}
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


    let config_dir =
        match initialize_user_files::initialize() {

            Ok(path) => path,

            Err(error) => {

                eprintln!(
                    "Screenshaver could not initialize its user files: {error}"
                );

                std::process::exit(
                    1
                );
            }
        };


    println!(
        "Screenshaver configuration directory: {}",
        config_dir.display()
    );


    let logfile =
        crate::locate_paths::runtime_log_path();


    let xfce_presentation_child =
        std::env::var_os(
            "XSCREENSAVER_WINDOW"
        ).is_some();


    if xfce_presentation_child {
        crate::logger::ensure_log_exists(
            &logfile
        );
    } else {
        crate::logger::reset_log(
            &logfile
        );
    }


    if xfce_presentation_child {

        match crate::manage_runtime_xfce::resident_runtime_active() {

            Ok(true) => {
                crate::logger::information(
                    &logfile,
                    "[LOCK] XFCE trusted saver child verified an active resident Screenshaver runtime.",
                );
            }


            Ok(false) => {
                crate::logger::information(
                    &logfile,
                    "[LOCK] XFCE trusted saver child found no active resident Screenshaver runtime; shader presentation will not start.",
                );

                return;
            }


            Err(error) => {
                crate::logger::warning(
                    &logfile,
                    &format!(
                        "[LOCK] Unable to verify XFCE resident runtime ownership; shader presentation will not start: {}",
                        error,
                    ),
                );

                return;
            }
        }


        match crate::present_screen_lock_xfce::detect_presentation_window(
            &logfile
        ) {
            Ok(window) => {
                println!(
                    "XFCE presentation window detected: 0x{:X}",
                    window
                );
            }

            Err(error) => {
                crate::logger::error(
                    &logfile,
                    &format!(
                        "[LOCK] XFCE presentation-window detection failed: {}",
                        error,
                    ),
                );

                eprintln!(
                    "XFCE presentation-window detection failed: {}",
                    error
                );
            }
        }

        return;
    }


    println!(
        "[MAIN] Loading configuration..."
    );


    // Policy rows are now authoritative in SQLite, so the database must be
    // available before load_config() hydrates per-shader policy state.
    let mut database_connection =
        match crate::evaluate_database::evaluate() {

            Ok(connection) => {
                connection
            }

            Err(error) => {

                eprintln!(
                    "[MAIN] DATABASE ERROR: {}",
                    error
                );


                crate::logger::error(
                    &logfile,
                    &format!(
                        "[DATABASE] Unable to prepare Screenshaver database: {}",
                        error,
                    ),
                );


                return;
            }
        };


    // Keep the managed shader inventory current before configuration is
    // hydrated from SQLite.  This is normal production startup behavior.
    match crate::reconcile_shaders::reconcile(
        &mut database_connection
    ) {
        Ok(outcome) => {
            crate::logger::information(
                &logfile,
                &format!(
                    "[DATABASE] Shader reconciliation complete: inserted={}, updated={}, unchanged={}, missing={}, unreadable={}, rejected={}",
                    outcome.inserted,
                    outcome.updated,
                    outcome.unchanged,
                    outcome.marked_missing,
                    outcome.marked_unreadable,
                    outcome.rejected,
                ),
            );
        }

        Err(error) => {
            eprintln!(
                "[MAIN] SHADER RECONCILIATION ERROR: {}",
                error
            );

            crate::logger::error(
                &logfile,
                &format!(
                    "[DATABASE] Shader reconciliation failed: {}",
                    error,
                ),
            );

            return;
        }
    }


    match crate::assign_shader_policies::offer_assignment_if_needed(
        &mut database_connection
    ) {
        Ok(
            crate::assign_shader_policies::AssignmentOutcome::NoPoliciesNeeded
        ) => {}

        Ok(
            crate::assign_shader_policies::AssignmentOutcome::Dismissed {
                shader_count,
            }
        ) => {
            crate::logger::information(
                &logfile,
                &format!(
                    "[POLICY] New-policy assignment dismissed; {} shader(s) remain without policies",
                    shader_count,
                ),
            );
        }

        Ok(
            crate::assign_shader_policies::AssignmentOutcome::Created {
                shader_count,
                policy_count,
                assignment,
            }
        ) => {
            crate::logger::information(
                &logfile,
                &format!(
                    "[POLICY] Created {} policy/policies for {} shader(s) using '{}'",
                    policy_count,
                    shader_count,
                    assignment.name(),
                ),
            );
        }

        Err(error) => {
            crate::logger::warning(
                &logfile,
                &format!(
                    "[POLICY] Unable to offer new-policy assignment: {}",
                    error,
                ),
            );
        }
    }


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


                crate::logger::error(
                    &logfile,
                    &format!(
                        "[CONFIG] Unable to load configuration: {}",
                        error,
                    ),
                );


                return;
            }
        };


    let mut cfg =
        result.config;


    crate::logger::set_enabled(
        cfg.debug_log
    );


    crate::logger::set_log_level(
        cfg.log_level
    );


    let desktop_environment =
        crate::detect_desktop_environment::detect();


    println!(
        "[MAIN] Desktop environment = {}",
        desktop_environment.name()
    );


    crate::logger::information(
        &logfile,
        &format!(
            "[SESSION] Desktop environment: {}",
            desktop_environment.name(),
        ),
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


    match command {

        crate::parse_arguments::Command::Control {
            shader_name,
        } => {

            let control_audio_backend =
                crate::audio_backend::create_backend()
                    .ok();


            let control_audio_bands =
                control_audio_backend
                    .as_ref()
                    .map(
                        |backend| {
                            backend.shared_bands()
                        }
                    );


            match crate::edit_shader::run(
                shader_name,
                control_audio_bands,
            ) {

                Ok(()) => {}

                Err(error) => {

                    eprintln!(
                        "[CONTROL CENTER] {}",
                        error
                    );


                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[CONTROL_CENTER] {}",
                            error,
                        ),
                    );
                }
            }


            drop(
                database_connection
            );

            return;
        }


        crate::parse_arguments::Command::Run
        | crate::parse_arguments::Command::Start => {}


        crate::parse_arguments::Command::Stop
        | crate::parse_arguments::Command::Help
        | crate::parse_arguments::Command::Version
        | crate::parse_arguments::Command::ConstructLockScreenKde
        | crate::parse_arguments::Command::ConstructLockScreenXfce => {

            unreachable!(
                "Database-independent command reached runtime startup"
            );
        }
    }


    // A configuration with both renderers disabled is valid, but there is no
    // useful resident runtime to start.  --control has already been handled
    // above, so ordinary Run/Start invocations can report the inactive state
    // and exit successfully before initializing audio, singleton/tray state,
    // or any rendering backend.
    if !cfg.screensaver_enabled
        && !cfg.wallpaper_enabled
    {
        eprintln!(
            "WARNING: Screensaver and wallpaper rendering are both disabled."
        );

        eprintln!(
            "Screenshaver has no active rendering functions and will exit."
        );

        eprintln!(
            "Run \"screenshaver --control\" to enable screensavers or wallpapers."
        );


        crate::logger::warning(
            &logfile,
            "[MAIN] Screensaver and wallpaper rendering are both disabled; no renderer will be started",
        );


        crate::logger::information(
            &logfile,
            "[MAIN] Screenshaver exiting normally because no rendering functions are enabled",
        );


        drop(
            database_connection
        );

        return;
    }


    // Audio is an optional runtime capability.  Failure to locate a usable
    // backend must never prevent Screenshaver from continuing normally.
    let audio_backend =
        match crate::audio_backend::create_backend() {

            Ok(backend) => {

                println!(
                    "[MAIN] Audio backend = {}",
                    backend.backend_name()
                );


                crate::logger::information(
                    &logfile,
                    &format!(
                        "[AUDIO] Backend ready: {}",
                        backend.backend_name(),
                    ),
                );


                Some(backend)
            }


            Err(error) => {

                println!(
                    "[AUDIO] No compatible audio backend available: {}",
                    error
                );


                crate::logger::warning(
                    &logfile,
                    &format!(
                        "[AUDIO] No compatible audio backend available: {}",
                        error,
                    ),
                );


                crate::logger::information(
                    &logfile,
                    "[AUDIO] Audio Bloom unavailable for this session",
                );


                None
            }
        };


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


                crate::logger::error(
                    &logfile,
                    &format!(
                        "[MAIN] Singleton acquisition failed: {}",
                        error,
                    ),
                );


                return;
            }
        };


    // Xfce may launch the trusted Screenshaver saver executable independently
    // of the resident application.  Establish an explicit runtime-ownership
    // marker only after the real Screenshaver singleton has been acquired.
    // The separately launched Xfce saver child requires this marker before it
    // is allowed to render shaders.
    let _xfce_runtime_session =
        if desktop_environment.is_xfce()
            && cfg.screen_lock_enabled
        {
            match crate::manage_runtime_xfce::XfceRuntimeSession::acquire(
                &logfile
            ) {
                Ok(session) => {
                    Some(
                        session
                    )
                }


                Err(error) => {
                    crate::logger::warning(
                        &logfile,
                        &format!(
                            "[LOCK] Unable to establish XFCE runtime ownership; XFCE shader lock presentation will remain unavailable: {}",
                            error,
                        ),
                    );

                    None
                }
            }
        } else {
            None
        };


    // GNOME lock-screen integration is runtime-owned. Establish the
    // authorization marker only after the resident Screenshaver singleton has
    // been acquired, then provision and activate the Shell extension. Any
    // failure here disables only Screenshaver's custom GNOME presentation:
    // GNOME's proven native secure-lock path remains available as the fallback.
    let gnome_runtime_session =
        if desktop_environment.is_gnome()
            && cfg.screen_lock_enabled
        {
            match crate::manage_gnome_extension::GnomeRuntimeSession::acquire(
                &logfile
            ) {
                Ok(session) => {
                    println!(
                        "[LOCK] GNOME runtime ownership established for this Screenshaver process."
                    );

                    crate::logger::information(
                        &logfile,
                        &format!(
                            "[LOCK] GNOME runtime ownership guard active: pid={} marker={} session={}",
                            session.pid(),
                            session.marker_path().display(),
                            session.session_id(),
                        ),
                    );

                    Some(session)
                }

                Err(error) => {
                    eprintln!(
                        "[LOCK] Unable to establish GNOME runtime ownership; stock GNOME lock presentation will be used: {}",
                        error
                    );

                    crate::logger::warning(
                        &logfile,
                        &format!(
                            "[LOCK] Unable to establish GNOME runtime ownership; stock GNOME lock presentation will be used: {}",
                            error,
                        ),
                    );

                    None
                }
            }
        } else {
            None
        };


    let mut gnome_extension_integration =
        None;

    if desktop_environment.is_gnome() {
        if cfg.screen_lock_enabled {
            if gnome_runtime_session.is_some() {
                match crate::manage_gnome_extension::GnomeExtensionIntegrationGuard::activate(
                    &logfile
                ) {
                    Ok(integration) => {
                        println!(
                            "[LOCK] Screenshaver GNOME Shell extension enabled for this runtime."
                        );

                        gnome_extension_integration =
                            Some(
                                integration
                            );
                    }

                    Err(error) => {
                        eprintln!(
                            "[LOCK] GNOME extension activation unavailable; stock GNOME lock presentation will be used: {}",
                            error
                        );

                        crate::logger::warning(
                            &logfile,
                            &format!(
                                "[LOCK] GNOME extension activation unavailable; stock GNOME lock presentation will be used: {}",
                                error,
                            ),
                        );
                    }
                }
            }
        } else {
            crate::manage_gnome_extension::disable_if_present(
                &logfile
            );
        }
    }


    // KDE lock-screen integration is runtime-owned. Do not touch KScreenLocker
    // until the resident Screenshaver singleton has been acquired; this keeps
    // --control and duplicate invocations from changing the active desktop.
    //
    // If locking is disabled, remove only stale Screenshaver-owned state left
    // by an abnormal previous termination. If locking is enabled, install a
    // fresh overlay and retain a guard that restores KDE on normal shutdown.
    let mut kde_lock_integration = None;

    if desktop_environment.is_kde_plasma() {
        if cfg.screen_lock_enabled {
            let lock_widget_config =
                crate::define_lock_screen_widget::LockScreenWidgetConfig::default();

            match crate::manage_screen_lock_kde::KdeLockIntegrationGuard::activate(
                &lock_widget_config
            ) {
                Ok((guard, status)) => {
                    println!(
                        "[LOCK] KDE Plasma lock-screen integration enabled for this Screenshaver runtime."
                    );

                    crate::logger::information(
                        &logfile,
                        &format!(
                            "[LOCK] KDE runtime lock-screen integration enabled: package_installed={} qml_installed={} screenshaver_selected={}",
                            status.shell_package_installed,
                            status.lockscreen_qml_installed,
                            status.screenshaver_selected,
                        ),
                    );

                    kde_lock_integration = Some(guard);
                }

                Err(error) => {
                    eprintln!(
                        "[LOCK] Unable to establish KDE Plasma lock-screen integration: {}",
                        error
                    );

                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[LOCK] Unable to establish KDE Plasma lock-screen integration: {}",
                            error,
                        ),
                    );

                    drop(database_connection);
                    return;
                }
            }
        } else {
            match crate::manage_screen_lock_kde::restore_stale_runtime_state() {
                Ok(_) => {
                    crate::logger::information(
                        &logfile,
                        "[LOCK] Screenshaver locking disabled; KDE lock-screen integration left inactive",
                    );
                }

                Err(error) => {
                    // A foreign same-ID user overlay is never Screenshaver's
                    // property to remove. Report it, but do not alter it or
                    // prevent ordinary non-locking Screenshaver operation.
                    crate::logger::warning(
                        &logfile,
                        &format!(
                            "[LOCK] Unable to remove stale Screenshaver KDE state; KDE was not modified: {}",
                            error,
                        ),
                    );
                }
            }
        }
    }

    let _kde_idle_lock_inhibitor =
        if desktop_environment.is_kde_plasma()
            && (cfg.screensaver_enabled || cfg.screen_lock_enabled)
        {

            match crate::manage_screen_lock_kde::KdeIdleLockInhibitor::acquire() {

                Ok(inhibitor) => {

                    println!(
                        "[SESSION] KDE native idle screen management inhibited while Screenshaver is running."
                    );


                    crate::logger::information(
                        &logfile,
                        &format!(
                            "[SESSION] KDE native idle screen management inhibited; cookie={}",
                            inhibitor.cookie(),
                        ),
                    );


                    Some(
                        inhibitor
                    )
                }


                Err(error) => {

                    eprintln!(
                        "[SESSION] Unable to inhibit KDE native idle screen management: {}",
                        error
                    );


                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[SESSION] Unable to inhibit KDE native idle screen management: {}",
                            error,
                        ),
                    );


                    crate::logger::error(
                        &logfile,
                        "[SESSION] Refusing to continue because KDE idle screen management could interrupt Screenshaver rendering",
                    );


                    drop(
                        database_connection
                    );

                    return;
                }
            }

        } else {

            None
        };


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


                crate::logger::information(
                    &logfile,
                    "[TRAY] System tray icon registered successfully",
                );


                Some(handle)
            }

            Err(error) => {

                eprintln!(
                    "[TRAY] System tray icon unavailable: {:?}",
                    error
                );


                crate::logger::warning(
                    &logfile,
                    &format!(
                        "[TRAY] System tray icon unavailable: {:?}",
                        error,
                    ),
                );


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


    //---------------------------------------------------------
    // Screen locking uses the same idle threshold as the
    // screensaver. load_config() has already enforced the
    // 60-second minimum whenever screen locking is enabled.
    //---------------------------------------------------------

    if cfg.screen_lock_enabled {
        println!(
            "[LOCK] Screen locking enabled; using screensaver idle timeout ({} seconds)",
            parsed_idle.duration.as_secs(),
        );

        crate::logger::information(
            &logfile,
            &format!(
                "[LOCK] Screen locking enabled; using screensaver idle timeout '{}' ({} seconds)",
                cfg.idle_timeout,
                parsed_idle.duration.as_secs(),
            ),
        );
    } else {
        crate::logger::information(
            &logfile,
            "[LOCK] Screen locking disabled",
        );
    }


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
            audio_bands:
                audio_backend
                    .as_ref()
                    .map(
                        |backend| {
                            backend.shared_bands()
                        }
                    ),
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
                                active_wallpaper.path,
                                active_wallpaper.policy_id,
                                audio_backend
                                    .as_ref()
                                    .map(
                                        |backend| {
                                            backend.shared_bands()
                                        }
                                    ),
                            )
                        }

                        None => {
                            crate::edit_shader::run(
                                None,
                                audio_backend
                                    .as_ref()
                                    .map(
                                        |backend| {
                                            backend.shared_bands()
                                        }
                                    ),
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

                    if cfg.screen_lock_enabled {
                        crate::logger::information(
                            &logfile,
                            "[LOCK] Screensaver idle threshold reached; engaging negotiated secure-lock backend",
                        );

                        println!(
                            "[MAIN] Screensaver idle threshold reached: engaging negotiated secure-lock backend"
                        );


                        let lock_result =
                            if desktop_environment
                                .is_kde_plasma()
                            {

                                drop(
                                    shader_manager
                                );


                                crate::manage_screen_lock_kde::run(
                                    &logfile,
                                    running.as_ref(),
                                    &wallpaper_control,
                                )

                            } else if desktop_environment
                                .is_xfce()
                            {

                                drop(
                                    shader_manager
                                );


                                crate::manage_screen_lock_xfce::run(
                                    &logfile,
                                    running.as_ref(),
                                    &wallpaper_control,
                                )

                            } else if desktop_environment
                                .is_gnome()
                            {

                                let presenter_result =
                                    match (gnome_runtime_session.as_ref(), gnome_extension_integration.as_ref()) {
                                        (Some(gnome_runtime_session), Some(_gnome_extension_integration)) => {
                                            crate::present_screen_lock_gnome::GnomeLockPresenter::start(
                                                &sdl,
                                                &logfile,
                                                gnome_runtime_session.session_id(),
                                                shader_manager,
                                                next_shader_interval,
                                                cfg.screensaver_speed_policy.clone(),
                                                cfg.global_rendered_fps,
                                                cfg.screensaver_fps_policy_entries.clone(),
                                                cfg.texture_policy.clone(),
                                                cfg.screensaver_postprocess_policy.clone(),
                                                audio_backend
                                                    .as_ref()
                                                    .map(
                                                        |backend| {
                                                            backend.shared_bands()
                                                        }
                                                    ),
                                                cfg.subtitles,
                                                cfg.subtitle_placement,
                                            )
                                        }

                                        _ => {
                                            Err(
                                                "GNOME custom presentation is unavailable because its runtime ownership or extension activation guard is missing"
                                                    .to_string()
                                            )
                                        }
                                    };


                                match presenter_result {
                                    Ok(mut presenter) => {
                                        let lock_logfile =
                                            logfile.clone();

                                        let lock_running =
                                            Arc::clone(
                                                &running
                                            );

                                        let lock_wallpaper_control =
                                            wallpaper_control.clone();


                                        match std::thread::Builder::new()
                                            .name(
                                                "screenshaver-gnome-secure-lock".to_string()
                                            )
                                            .spawn(
                                                move || {
                                                    crate::manage_screen_lock_gnome::run(
                                                        &lock_logfile,
                                                        lock_running.as_ref(),
                                                        &lock_wallpaper_control,
                                                    )
                                                }
                                            )
                                        {
                                            Ok(lock_thread) => {
                                                let render_result =
                                                    presenter.run_until(
                                                        || {
                                                            lock_thread.is_finished()
                                                        }
                                                    );


                                                if let Err(error) =
                                                    render_result
                                                {
                                                    crate::logger::warning(
                                                        &logfile,
                                                        &format!(
                                                            "[LOCK] GNOME shader presentation stopped with an error while secure lock remained active: {}",
                                                            error,
                                                        ),
                                                    );

                                                    eprintln!(
                                                        "[LOCK] GNOME shader presentation stopped with an error: {}",
                                                        error,
                                                    );
                                                }


                                                drop(
                                                    presenter
                                                );


                                                match lock_thread.join() {
                                                    Ok(result) => {
                                                        result
                                                    }

                                                    Err(_) => {
                                                        Err(
                                                            "GNOME secure-lock worker thread panicked"
                                                                .to_string()
                                                        )
                                                    }
                                                }
                                            }

                                            Err(error) => {
                                                crate::logger::error(
                                                    &logfile,
                                                    &format!(
                                                        "[LOCK] Unable to start GNOME secure-lock worker thread: {}",
                                                        error,
                                                    ),
                                                );

                                                eprintln!(
                                                    "[LOCK] Unable to start GNOME secure-lock worker thread: {}",
                                                    error,
                                                );

                                                Err(
                                                    format!(
                                                        "Unable to start GNOME secure-lock worker thread: {}",
                                                        error,
                                                    )
                                                )
                                            }
                                        }
                                    }

                                    Err(error) => {
                                        crate::logger::error(
                                            &logfile,
                                            &format!(
                                                "[LOCK] GNOME shader presentation could not be started: {}",
                                                error,
                                            ),
                                        );

                                        eprintln!(
                                            "[LOCK] GNOME shader presentation could not be started: {}",
                                            error,
                                        );


                                        // Preserve the already-proven secure-lock path even
                                        // when visual presentation initialization fails.
                                        crate::manage_screen_lock_gnome::run(
                                            &logfile,
                                            running.as_ref(),
                                            &wallpaper_control,
                                        )
                                    }
                                }

                            } else {

                                crate::manage_screen_lock::run(
                                    &logfile,
                                    running.as_ref(),
                                    &wallpaper_control,
                                    shader_manager,
                                    next_shader_interval,
                                    cfg.screensaver_speed_policy.clone(),
                                    cfg.global_rendered_fps,
                                    cfg.screensaver_fps_policy_entries.clone(),
                                    cfg.texture_policy.clone(),
                                    cfg.screensaver_postprocess_policy.clone(),
                                    audio_backend
                                        .as_ref()
                                        .map(
                                            |backend| {
                                                backend.shared_bands()
                                            }
                                        ),
                                    cfg.subtitles,
                                    cfg.subtitle_placement,
                                )
                            };


                        match lock_result {
                            Ok(()) => {
                                crate::logger::information(
                                    &logfile,
                                    "[LOCK] Authenticated secure-lock session completed",
                                );
                            }

                            Err(error) => {
                                eprintln!(
                                    "[LOCK] Secure screen lock could not be engaged: {}",
                                    error,
                                );

                                crate::logger::error(
                                    &logfile,
                                    &format!(
                                        "[LOCK] Secure screen lock could not be engaged: {}",
                                        error,
                                    ),
                                );

                                wallpaper_control.resume_and_wait_for_frame(
                                    running.as_ref()
                                );
                            }
                        }


                        break 'screensaver_session;
                    }


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
                            audio_backend
                                .as_ref()
                                .map(
                                    |backend| {
                                        backend.shared_bands()
                                    }
                                ),
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
                                    shader_path.clone(),
                                    audio_backend
                                        .as_ref()
                                        .map(
                                            |backend| {
                                                backend.shared_bands()
                                            }
                                        ),
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


    if let Some(mut integration) =
        gnome_extension_integration.take()
    {
        match integration.deactivate() {
            Ok(()) => {
                crate::logger::information(
                    &logfile,
                    "[LOCK] GNOME Shell extension deactivated on Screenshaver shutdown",
                );
            }

            Err(error) => {
                crate::logger::warning(
                    &logfile,
                    &format!(
                        "[LOCK] Unable to deactivate GNOME Shell extension during shutdown: {}",
                        error,
                    ),
                );
            }
        }
    }


    if let Some(mut integration) = kde_lock_integration.take() {
        match integration.deactivate() {
            Ok(_) => {
                crate::logger::information(
                    &logfile,
                    "[LOCK] KDE Plasma lock-screen integration restored on Screenshaver shutdown",
                );
            }

            Err(error) => {
                crate::logger::error(
                    &logfile,
                    &format!(
                        "[LOCK] Unable to restore KDE Plasma lock-screen integration during shutdown: {}",
                        error,
                    ),
                );
            }
        }
    }


    println!(
        "[MAIN] Pipeline complete."
    );


    crate::logger::information(
        &logfile,
        "[MAIN] Pipeline complete",
    );


    drop(
        database_connection
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

