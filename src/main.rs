mod initialize_user_files;
mod startup_checks;
mod load_config;
mod singleton;
mod logger;
mod parse_mode;
mod parse_interval;
mod parse_duration;
mod parse_arguments;
mod renderer_status;
mod parse_subtitle_placement;
mod parse_texture_specification;

mod define_constants;
mod locate_paths;
mod manage_runtime_state;

mod query_session;
mod session_backend;
mod audio_backend;
mod analyze_audio;

mod manage_configuration;
mod manage_shader;
mod manage_screen_lock;
mod manage_screen_lock_gnome;
mod manage_screen_lock_xfce;
mod manage_screen_lock_mate;
mod manage_runtime_xfce;
mod manage_runtime_mate;
mod test_mate_lock;
mod manage_gnome_extension;
mod present_screen_lock_gnome;
mod present_screen_lock_xfce;
mod lock_screen_widget;
mod define_lock_screen_widget;
mod construct_lock_screen_xfce;
mod construct_lock_screen_mate;
mod manage_screen_lock_kde;
mod manage_windowshader_kde;
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
mod tray_icon_x11;

mod define_operation;
mod define_wallpaper;
mod configure_wallpaper;
mod control_wallpaper;
mod control_windowshader;
mod locate_wallpaper;
mod manage_wallpaper;
mod manage_windowshader;
mod manage_wallpaper_runtime;
mod manage_windowshader_runtime;
mod manage_lyrics;
mod notify_wallpaper;
mod render_wallpaper;
mod wayland_wallpaper;
mod wayland_windowshader;
mod wayland_render_core;
mod wallpaper_backend;
mod windowshader_backend;
mod x11_connection;
mod x11_wallpaper;
mod x11_windowshader;
mod glx_context;
mod fps_monitor;
mod monitor_lock_presentation;

mod postprocess_shader;
mod render_passthrough;
mod render_fxaa;
mod render_dithering;
mod render_bloom;
mod render_audio_motion;
mod select_render_precision;

mod database_factory;
mod initialize_database;
mod evaluate_database;
mod open_database;
mod validate_database;
mod migrate_database;
mod database_migration;
mod test_schema_reader;
mod test_schema_reconstruction;
mod test_schema_migration_failures;
mod test_database_cutover_recovery;
mod test_migration_coordinator;
mod hash_shader;

mod qbe_layout;
mod parse_qbe;
mod query_database;

mod create_wayland_lock_context;
mod display_lock_authentication;
mod authenticate_user;

mod manage_playlists;
mod test_playlists;
mod test_lyrics;
mod test_lyric_translation;
mod test_localization;
mod manage_localization;
mod audit_translation;
mod test_render_benchmark;
mod test_audio_motion;
mod test_libdecor;

mod import_data;
mod export_data;
mod manage_backup;
mod compare_databases;

mod runtime_source_version;


use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::time::Duration;




fn format_compile_shader_text(
    key: &str,
    params: &[(&str, &str)],
) -> String {

    if params.is_empty() {
        crate::manage_localization::runtime_text(
            key
        )
    } else {
        crate::manage_localization::runtime_text_with_params(
            key,
            params,
        )
    }
}


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


        crate::parse_arguments::Command::AuditTranslation {
            locale,
            module,
            all_locales,
            online,
        } => {

            match crate::audit_translation::run(
                locale.as_deref(),
                module.as_deref(),
                *all_locales,
                *online,
            ) {
                Ok(true) => {}
                Ok(false) => std::process::exit(1),
                Err(error) => {
                    eprintln!("[TRANSLATION AUDIT] ERROR: {}", error);
                    std::process::exit(2);
                }
            }

            return;
        }


        crate::parse_arguments::Command::CompareDatabases {
            database_a,
            database_b,
            exclude_metadata,
            exclude_local_config,
        } => {

            match crate::compare_databases::compare(
                database_a,
                database_b,
                *exclude_metadata,
                *exclude_local_config,
            ) {
                Ok(summary) => {
                    if summary.differences == 0 {
                        println!(
                            "No differences found."
                        );
                    } else {
                        println!(
                            "\nComparison complete: {} difference(s) found.",
                            summary.differences,
                        );

                        std::process::exit(
                            1
                        );
                    }
                }

                Err(error) => {
                    eprintln!(
                        "[DATABASE COMPARE] {}",
                        error
                    );

                    std::process::exit(
                        2
                    );
                }
            }


            return;
        }


        crate::parse_arguments::Command::TestLibdecor => {
            if let Err(error) = crate::test_libdecor::run() {
                eprintln!("[LIBDECOR TEST] FAILED: {error}");
                std::process::exit(1);
            }
            return;
        }

        crate::parse_arguments::Command::TestLibdecorBridge => {
            if let Err(error) = crate::test_libdecor::run_bridge() {
                eprintln!("[LIBDECOR BRIDGE TEST] FAILED: {error}");
                std::process::exit(1);
            }
            return;
        }

        crate::parse_arguments::Command::TestWallpaperProtocol => {
            if let Err(error) = crate::control_wallpaper::run_protocol_tests() {
                eprintln!("[WALLPAPER PROTOCOL TEST] FAILED: {error}");
                std::process::exit(1);
            }
            return;
        }

        crate::parse_arguments::Command::TestSchemaReader {
            database_path,
        } => {

            match crate::test_schema_reader::run(
                database_path.as_deref()
            ) {

                Ok(()) => {}

                Err(error) => {

                    eprintln!(
                        "[SCHEMA READER TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            return;
        }


        crate::parse_arguments::Command::TestSchemaReconstruction {
            source_path,
            destination_path,
        } => {

            match crate::test_schema_reconstruction::run(
                source_path,
                destination_path,
            ) {

                Ok(()) => {}

                Err(error) => {

                    eprintln!(
                        "[SCHEMA RECONSTRUCTION TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            return;
        }


        crate::parse_arguments::Command::TestSchemaMigrationFailures {
            fixture_path,
            work_directory,
        } => {

            match crate::test_schema_migration_failures::run(
                fixture_path,
                work_directory,
            ) {

                Ok(()) => {}

                Err(error) => {

                    eprintln!(
                        "[SCHEMA MIGRATION FAILURE TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            return;
        }


        crate::parse_arguments::Command::TestDatabaseCutoverRecovery => {

            match crate::test_database_cutover_recovery::run() {

                Ok(()) => {}

                Err(error) => {

                    eprintln!(
                        "[CUTOVER RECOVERY TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }


            return;
        }


        crate::parse_arguments::Command::TestMigrationCoordinator {
            database_path,
        } => {

            match crate::test_migration_coordinator::run(
                database_path.as_deref()
            ) {

                Ok(()) => {}

                Err(error) => {
                    eprintln!(
                        "[MIGRATION COORDINATOR TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            return;
        }


        crate::parse_arguments::Command::TestLyricTranslation { locale } => {
            if let Err(error) = crate::test_lyric_translation::run(locale) {
                eprintln!("[LYRIC TRANSLATION TEST] FAILED: {}", error);
                std::process::exit(1);
            }
            return;
        }

        crate::parse_arguments::Command::TestLyrics => {

            match crate::test_lyrics::run() {

                Ok(()) => {}

                Err(error) => {

                    eprintln!(
                        "[LYRICS TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            }


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


            match crate::manage_screen_lock_kde::install() {

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


        crate::parse_arguments::Command::TestMateLock => {
            if let Err(error) = crate::test_mate_lock::run() {
                eprintln!("[MATE TEST] FAILED: {error}");
                std::process::exit(1);
            }
            return;
        }

        crate::parse_arguments::Command::ConstructLockScreenMate => {
            let desktop = crate::detect_desktop_environment::detect();
            if !desktop.is_mate() {
                eprintln!(
                    "MATE lock-screen configuration was not modified: detected desktop is {}.",
                    desktop.name()
                );
                std::process::exit(1);
            }

            match crate::construct_lock_screen_mate::configure_user() {
                Ok(status) => {
                    println!("MATE Screensaver available: {}", status.mate_screensaver_available);
                    println!("Trusted MATE presenter installed: {}", status.trusted_presenter_installed);
                    println!("Screenshaver MATE theme registered: {}", status.saver_desktop_registered);
                    if !status.ready_for_runtime() {
                        eprintln!("MATE theme registration did not verify successfully.");
                        std::process::exit(1);
                    }
                    println!("MATE theme registered; native screensaver selection unchanged.");
                    println!("Production runtime activation is not yet implemented.");
                }
                Err(error) => {
                    eprintln!("Unable to configure MATE lock-screen theme: {error}");
                    std::process::exit(1);
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
        | crate::parse_arguments::Command::RunRendererStatus
        | crate::parse_arguments::Command::Start
        | crate::parse_arguments::Command::Control { .. }
        | crate::parse_arguments::Command::TestWallpaperProtocol
        | crate::parse_arguments::Command::TestPlaylists
        | crate::parse_arguments::Command::TestLocalization
        | crate::parse_arguments::Command::BenchmarkRender { .. }
        | crate::parse_arguments::Command::TestAudioMotion { .. }
        | crate::parse_arguments::Command::TestReverseAnimation { .. }
        | crate::parse_arguments::Command::ResetIdleTimeout { .. } => {}
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


    if let Err(error) = crate::manage_backup::ensure_directory() {
        eprintln!("[MAIN] BACKUP DIRECTORY ERROR: {}", error);
        std::process::exit(1);
    }


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


    // Capture MATE theme launches even if the native saver does not supply
    // XSCREENSAVER_WINDOW. This runs only in the explicitly tagged test child.
    if std::env::var("SCREENSHAVER_MATE_PRESENTATION_TEST").as_deref() == Ok("1") {
        crate::test_mate_lock::diagnostic(&format!(
            "Child entered main: desktop={}, session={:?}, XSCREENSAVER_WINDOW={:?}",
            crate::detect_desktop_environment::detect().name(),
            std::env::var("XDG_SESSION_TYPE").ok(),
            std::env::var("XSCREENSAVER_WINDOW").ok(),
        ));
    }

    if xfce_presentation_child {

        // MATE's screensaver engine child may not inherit desktop/session
        // identification variables. The explicit test launcher flag selects
        // this branch; presenter_authorized() still requires a live test owner.
        // The X11 presenter subsequently validates the supplied window.
        if std::env::var("SCREENSHAVER_MATE_PRESENTATION_TEST").as_deref() == Ok("1")
        {
            crate::test_mate_lock::diagnostic("Child entered MATE presenter authorization");
            match crate::test_mate_lock::presenter_authorized() {
                Ok(true) => {
                    crate::test_mate_lock::diagnostic("Authorization PASS; invoking X11/GLX presenter");
                    crate::logger::information(&logfile,
                        "[MATE TEST] Authorized presenter started");
                    let presentation = crate::present_screen_lock_xfce::detect_presentation_window(&logfile);
                    crate::test_mate_lock::diagnostic(&format!("GLX presenter returned: {:?}", presentation));
                    if let Err(error) = presentation {
                        eprintln!("[MATE TEST] GLX presenter failed: {error}");
                        crate::logger::error(&logfile,
                            &format!("[MATE TEST] GLX presenter failed: {error}"));
                    }
                }
                Ok(false) => {
                    crate::test_mate_lock::diagnostic("Authorization DENIED: no live owner");
                    eprintln!("[MATE TEST] No live test owner; refusing presentation");
                }
                Err(error) => {
                    crate::test_mate_lock::diagnostic(&format!("Authorization ERROR: {error}"));
                    eprintln!("[MATE TEST] Authorization error: {error}");
                },
            }
            return;
        }

        // Production MATE presenter selection is explicit. The trusted MATE
        // launcher supplies this tag; the tag alone is never authorization.
        // A live resident PID/start-time marker is also mandatory. This path
        // does not change MATE's selected theme or activate its runtime.
        if std::env::var("SCREENSHAVER_MATE_PRESENTATION").as_deref() == Ok("1") {
            match crate::manage_runtime_mate::resident_runtime_active() {
                Ok(true) => {
                    crate::logger::information(
                        &logfile,
                        "[LOCK] MATE production presenter verified resident runtime ownership",
                    );
                    if let Err(error) =
                        crate::present_screen_lock_xfce::detect_presentation_window(&logfile)
                    {
                        crate::logger::error(
                            &logfile,
                            &format!("[LOCK] MATE production presentation failed: {error}"),
                        );
                        eprintln!("MATE presentation failed: {error}");
                    }
                }
                Ok(false) => {
                    crate::logger::warning(
                        &logfile,
                        "[LOCK] MATE production presenter denied: no live resident runtime owner",
                    );
                }
                Err(error) => {
                    crate::logger::warning(
                        &logfile,
                        &format!("[LOCK] MATE production presenter authorization failed: {error}"),
                    );
                }
            }
            return;
        }

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


    // Localization catalogs are factory-owned current-executable data.
    // Synchronize them on database-dependent startup so an application update
    // can add or revise supported languages and translations without requiring
    // a database schema migration.
    if let Err(error) =
        crate::database_factory::synchronize_localization_catalog(
            &mut database_connection
        )
    {
        eprintln!(
            "[MAIN] LOCALIZATION CATALOG ERROR: {}",
            error
        );

        crate::logger::error(
            &logfile,
            &format!(
                "[LOCALIZATION] Unable to synchronize factory catalog: {}",
                error,
            ),
        );

        return;
    }


    // --test-localization exercises the Schema-2 localization catalog after
    // normal database preparation, but before shader reconciliation or runtime
    // startup.  Existing configuration files without [language] default to
    // en-US.
    if let crate::parse_arguments::Command::TestLocalization = &command {

        let cfg_path =
            crate::locate_paths::config_path();

        let locale =
            match crate::load_config::load_locale(
                &cfg_path
            ) {
                Ok(locale) => locale,

                Err(error) => {
                    eprintln!(
                        "[LOCALIZATION TEST] FAILED: {}",
                        error
                    );

                    std::process::exit(
                        1
                    );
                }
            };

        match crate::test_localization::run(
            &database_connection,
            &locale,
        ) {
            Ok(()) => {}

            Err(error) => {
                eprintln!(
                    "[LOCALIZATION TEST] FAILED: {}",
                    error
                );

                std::process::exit(
                    1
                );
            }
        }

        drop(
            database_connection
        );

        return;
    }


    // --test-playlists is a temporary developer command. Run it only after
    // the database has been prepared, and before shader reconciliation or
    // normal runtime/configuration startup.
    if let crate::parse_arguments::Command::TestPlaylists = &command {

        match crate::test_playlists::run() {

            Ok(()) => {
                println!(
                    "[PLAYLIST TEST] All Playlist database-management tests passed."
                );
            }

            Err(error) => {
                eprintln!(
                    "[PLAYLIST TEST] FAILED: {}",
                    error
                );

                std::process::exit(
                    1
                );
            }
        }


        drop(
            database_connection
        );

        return;
    }


    // --reset-idle-timeout is a recovery command.  Handle it after the
    // database has been prepared, but before shader reconciliation or normal
    // configuration loading so an invalid existing timeout cannot block repair.
    if let crate::parse_arguments::Command::ResetIdleTimeout { value } = &command {
        let cfg_path = crate::locate_paths::config_path();

        let screen_lock_enabled =
            match crate::load_config::load_screen_lock_enabled(&cfg_path) {
                Ok(enabled) => enabled,
                Err(error) => {
                    eprintln!("[CONFIG] Unable to determine screen-lock state: {}", error);
                    return;
                }
            };

        match crate::manage_configuration::reset_screensaver_idle_timeout(
            value,
            screen_lock_enabled,
        ) {
            Ok((stored_value, stored_unit, clamped)) => {
                if clamped {
                    println!(
                        "[CONFIG] Requested idle timeout '{}' is below the 60-second minimum required when screen locking is enabled; stored 60 seconds instead.",
                        value,
                    );
                } else {
                    println!(
                        "[CONFIG] Screensaver idle timeout reset to {} {}.",
                        stored_value,
                        stored_unit,
                    );
                }
            }

            Err(error) => {
                eprintln!("[CONFIG] Unable to reset screensaver idle timeout: {}", error);
            }
        }

        return;
    }


    // The first-run policy assignment dialog runs before load_config().
    // Initialize runtime translations now, after factory catalog synchronization,
    // so every startup dialog can resolve its localization keys.
    let startup_locale = match crate::load_config::load_locale(
        &crate::locate_paths::config_path()
    ) {
        Ok(locale) => locale,
        Err(error) => {
            eprintln!("[MAIN] LOCALIZATION CONFIG ERROR: {}", error);
            crate::logger::error(&logfile, &format!(
                "[LOCALIZATION] Unable to load startup locale: {}", error
            ));
            return;
        }
    };

    if let Err(error) =
        crate::manage_localization::initialize_runtime(
            &database_connection,
            &startup_locale,
        )
    {
        eprintln!(
            "[MAIN] LOCALIZATION ERROR: {}",
            error
        );


        crate::logger::error(
            &logfile,
            &format!(
                "[LOCALIZATION] Unable to initialize runtime localization: {}",
                error,
            ),
        );


        return;
    }


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


    // Assignment management is initiated from the Control Center, not on
    // every startup. Existing databases are never interrupted by a modal.

    // Automatic backup runs only after database preparation, shader
    // reconciliation, and required policy assignment have completed. A failed
    // backup never advances app_defaults.last_backup, so the next startup can
    // retry it.
    match crate::manage_backup::run_scheduled_backup_if_due() {
        Ok(Some(path)) => {
            crate::logger::information(
                &logfile,
                &format!("[BACKUP] Automatic full backup created: {}", path.display()),
            );
        }
        Ok(None) => {}
        Err(error) => {
            crate::logger::warning(
                &logfile,
                &format!("[BACKUP] Automatic full backup failed: {}", error),
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




    if let Err(error) =
        crate::compile_shader::set_text_formatter(
            format_compile_shader_text
        )
    {
        eprintln!(
            "[MAIN] COMPILE-SHADER LOCALIZATION ERROR: {}",
            error
        );

        crate::logger::error(
            &logfile,
            &format!(
                "[LOCALIZATION] Unable to initialize compile-shader text localization: {}",
                error,
            ),
        );

        return;
    }


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

        crate::parse_arguments::Command::TestAudioMotion {
            shader_path,
        } => {

            match crate::test_audio_motion::run(
                &shader_path,
                &cfg,
            ) {
                Ok(()) => {}

                Err(error) => {
                    eprintln!(
                        "[AUDIO MOTION TEST] FAILED: {}",
                        error
                    );

                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[AUDIO_MOTION_TEST] {}",
                            error,
                        ),
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            drop(
                database_connection
            );

            return;
        }


        crate::parse_arguments::Command::TestReverseAnimation {
            shader_path,
        } => {

            match crate::test_render_benchmark::run_reverse_animation(
                &shader_path,
                &cfg,
            ) {
                Ok(()) => {}

                Err(error) => {
                    eprintln!(
                        "[REVERSE ANIMATION TEST] FAILED: {}",
                        error
                    );

                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[REVERSE_ANIMATION_TEST] {}",
                            error,
                        ),
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            drop(
                database_connection
            );

            return;
        }


        crate::parse_arguments::Command::BenchmarkRender {
            shader_path,
        } => {

            match crate::test_render_benchmark::run(
                &shader_path,
                &cfg,
            ) {
                Ok(()) => {}

                Err(error) => {
                    eprintln!(
                        "[RENDER BENCHMARK] FAILED: {}",
                        error
                    );

                    crate::logger::error(
                        &logfile,
                        &format!(
                            "[RENDER_BENCHMARK] {}",
                            error,
                        ),
                    );

                    std::process::exit(
                        1
                    );
                }
            }

            drop(
                database_connection
            );

            return;
        }


        crate::parse_arguments::Command::Control {
            shader_name,
            show_renderer_status,
        } => {
            let _diagnostic_session = if show_renderer_status {
                match crate::renderer_status::begin_session(false) {
                    Ok(session) => Some(session),
                    Err(error) => {
                        eprintln!("[CONTROL CENTER] Unable to enable renderer diagnostic: {error}");
                        return;
                    }
                }
            } else {
                None
            };

            // The Control Center receives the process-local band handle, but
            // this alone does not open PulseAudio.  Audio capture starts only
            // when the editor actually renders an Audio Bloom policy.
            let control_audio_bands =
                Some(
                    crate::audio_backend::shared_audio_bands()
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
        | crate::parse_arguments::Command::RunRendererStatus
        | crate::parse_arguments::Command::Start => {}


        crate::parse_arguments::Command::Stop
        | crate::parse_arguments::Command::Help
        | crate::parse_arguments::Command::CompareDatabases { .. }
        | crate::parse_arguments::Command::Version
        | crate::parse_arguments::Command::TestWallpaperProtocol
        | crate::parse_arguments::Command::TestPlaylists
        | crate::parse_arguments::Command::TestLibdecor
        | crate::parse_arguments::Command::TestLibdecorBridge
        | crate::parse_arguments::Command::TestLyrics
        | crate::parse_arguments::Command::TestLyricTranslation { .. }
        | crate::parse_arguments::Command::TestLocalization
        | crate::parse_arguments::Command::TestReverseAnimation { .. }
        | crate::parse_arguments::Command::AuditTranslation { .. }
        | crate::parse_arguments::Command::TestSchemaReader { .. }
        | crate::parse_arguments::Command::TestSchemaReconstruction { .. }
        | crate::parse_arguments::Command::TestSchemaMigrationFailures { .. }
        | crate::parse_arguments::Command::TestDatabaseCutoverRecovery
        | crate::parse_arguments::Command::TestMigrationCoordinator { .. }
        | crate::parse_arguments::Command::ConstructLockScreenKde
        | crate::parse_arguments::Command::ConstructLockScreenXfce
        | crate::parse_arguments::Command::ConstructLockScreenMate
        | crate::parse_arguments::Command::TestMateLock
        | crate::parse_arguments::Command::ResetIdleTimeout { .. } => {

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
        && !cfg.windowshader_enabled
    {
        eprintln!(
            "WARNING: Screensaver, wallpaper, and windowshader rendering are all disabled."
        );

        eprintln!(
            "Screenshaver has no active rendering functions and will exit."
        );

        eprintln!(
            "Run \"screenshaver --control\" to configure rendering targets."
        );


        crate::logger::warning(
            &logfile,
            "[MAIN] All three rendering targets are disabled; no renderer will be started",
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


    // Warn before acquiring the singleton or initializing runtime resources.
    // Exit terminates normally without modifying persistent configuration.
    let wallpaper_enabled_this_launch = cfg.wallpaper_enabled;
    let lxde_wallpaper_fullscreen = cfg.wallpaper_display_format
        == crate::manage_configuration::WallpaperDisplayFormat::FullScreen;
    let lxde_session = desktop_environment.is_lxde();
    let lxde_x11 = std::env::var("XDG_SESSION_TYPE")
        .map(|value| value.trim().eq_ignore_ascii_case("x11"))
        .unwrap_or(false);
    let lxde_pcmanfm = if wallpaper_enabled_this_launch
        && lxde_wallpaper_fullscreen && lxde_session && lxde_x11 {
        crate::detect_desktop_environment::pcmanfm_manages_x11_desktop()
    } else {
        false
    };
    crate::logger::information(&logfile, &format!(
        "[WALLPAPER] LXDE compatibility check: enabled={} display_format={:?} fullscreen={} desktop={} lxde={} x11={} pcmanfm_desktop={}",
        wallpaper_enabled_this_launch, cfg.wallpaper_display_format,
        lxde_wallpaper_fullscreen, desktop_environment.name(),
        lxde_session, lxde_x11, lxde_pcmanfm,
    ));
    if wallpaper_enabled_this_launch
        && lxde_wallpaper_fullscreen && lxde_session && lxde_x11 && lxde_pcmanfm
    {
        let title = crate::manage_localization::runtime_text(
            "warning.desktop_icon_compatibility_title"
        );
        let message = "Screenshaver has detected an LXDE desktop running under X11. \
PCManFM manages desktop icons and the desktop background together. \
When Screenshaver renders an animated wallpaper, your desktop icons will be obscured.\n\n\
Your icons and files will not be deleted or modified. \
They will become visible again when wallpaper rendering stops.";
        match crate::display_message::confirm_warning(
            &title, message, "Continue", "Exit",
        ) {
            Ok(true) => {}
            Ok(false) => {
                crate::logger::information(&logfile,
                    "[WALLPAPER] LXDE desktop icon warning declined; Screenshaver exiting normally");
                drop(database_connection);
                return;
            }
            Err(error) => {
                crate::logger::warning(&logfile, &format!(
                    "[WALLPAPER] Unable to display LXDE compatibility warning; Screenshaver exiting without wallpaper: {}", error));
                drop(database_connection);
                return;
            }
        }
    }

    // LXDE under X11 has no supported Screenshaver secure-lock presentation.
    // This restriction applies only to this launch; preserve the saved setting.
    let lxde_x11_lock_unsupported = lxde_session && lxde_x11;
    if lxde_x11_lock_unsupported && cfg.screen_lock_enabled {
        let message = "Screenshaver does not support screen locking in the current LXDE/X11 desktop environment.\n\n\
Screensaver and wallpaper functionality will remain available. Screen locking must be managed separately by your desktop's native locking application.";
        crate::logger::warning(
            &logfile,
            "[LOCK] LXDE/X11 screen locking is unsupported; suppressing Screenshaver secure locking for this launch. Normal screensaver rendering remains available; saved configuration is unchanged.",
        );
        if let Err(error) = crate::display_message::show_warning(
            &crate::manage_localization::runtime_text(
                "warning.screen_locking_unavailable_title"
            ),
            message,
        ) {
            crate::logger::warning(&logfile, &format!(
                "[LOCK] Unable to display LXDE/X11 screen-lock compatibility warning: {}",
                error,
            ));
        }
    }

    // Keep one stable audio-band handle for every renderer in this process.
    // Accessing it does not initialize PulseAudio or create a capture stream;
    // active renderers demand Audio Bloom capture only while their current
    // policy actually requires it.
    let audio_bands =
        Some(
            crate::audio_backend::shared_audio_bands()
        );


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


    // The singleton is now owned by this process: a rejected duplicate launch
    // cannot alter diagnostic state. The guard lives through resident shutdown.
    let _renderer_diagnostic_session =
        if matches!(&command, crate::parse_arguments::Command::RunRendererStatus) {
            match crate::renderer_status::begin_session(true) {
                Ok(session) => Some(session),
                Err(error) => {
                    eprintln!("[MAIN] Unable to enable renderer diagnostic: {error}");
                    return;
                }
            }
        } else {
            crate::renderer_status::disable_for_new_runtime();
            None
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


    // GNOME shader lock presentation is runtime-owned. Establish the
    // authorization marker only after the resident Screenshaver singleton has
    // been acquired, then provision and activate the Shell extension. GNOME
    // remains responsible for secure locking, input, authentication, and
    // authenticated unlock. Failure here disables only Screenshaver's shader
    // presentation; GNOME's native lock screen remains available.
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


    // The GNOME Shell extension serves both lock presentation and windowshader
    // geometry. Keep it active independently of the secure-lock setting.
    // GnomeRuntimeSession above still controls lock-screen authorization.
    let mut gnome_extension_integration = None;

    if desktop_environment.is_gnome() {
        match crate::manage_gnome_extension::GnomeExtensionIntegrationGuard::activate(
            &logfile
        ) {
            Ok(integration) => {
                println!("[GNOME] Screenshaver Shell extension active for this session.");
                gnome_extension_integration = Some(integration);
            }
            Err(error) => {
                eprintln!(
                    "[GNOME] Shell extension activation unavailable; GNOME integration may be limited: {}",
                    error
                );
                crate::logger::warning(
                    &logfile,
                    &format!("[GNOME] Shell extension activation unavailable: {}", error),
                );
            }
        }
    }


    // KDE shader lock presentation is runtime-owned. Do not touch KScreenLocker
    // until the resident Screenshaver singleton has been acquired; this keeps
    // --control and duplicate invocations from changing the active desktop.
    // KScreenLocker remains responsible for secure locking, input,
    // authentication, and authenticated unlock.
    //
    // If locking is disabled, remove only stale Screenshaver-owned state left
    // by an abnormal previous termination. If locking is enabled, install the
    // Screenshaver presentation underlay and retain a guard that restores KDE
    // on normal shutdown.
    let mut kde_lock_integration = None;

    if desktop_environment.is_kde_plasma() {
        if cfg.screen_lock_enabled {
            match crate::manage_screen_lock_kde::KdeLockIntegrationGuard::activate() {
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

    // Stage D: both presentation targets have independent supervisors,
    // backends, policy selection, and pause/reload control handles.
    // The shared PresentationRuntimeControl coordinates handoffs.
    let windowshader_enabled = cfg.windowshader_enabled;
    if cfg.wallpaper_enabled && windowshader_enabled {
        crate::logger::information(
            &logfile,
            "[PRESENTATION] Concurrent Wallpaper and Windowshader supervisors enabled",
        );
    }

    let tray_status =
        crate::tray_icon::TrayStatusControl::new(
            cfg.wallpaper_enabled || windowshader_enabled,
            cfg.wallpaper_enabled,
        );


    // LXPanel's traditional notification area uses XEmbed, not KSNI.
    // Keep the existing KSNI tray unchanged on all other desktops.
    let use_xembed_tray = lxde_session && lxde_x11;
    let _x11_tray_handle = if use_xembed_tray {
        match crate::tray_icon_x11::start(
            &logfile,
            tray_command_sender.clone(),
            crate::tray_icon::TrayStatus {
                screensaver_enabled: cfg.screensaver_enabled,
                windowshader_enabled,
                wallpaper: tray_status.clone(),
            },
        ) {
            Ok(handle) => {
                crate::logger::information(&logfile,
                    "[TRAY/X11] LXDE XEmbed tray backend started");
                Some(handle)
            }
            Err(error) => {
                crate::logger::warning(&logfile,
                    &format!("[TRAY/X11] XEmbed tray unavailable: {}", error));
                None
            }
        }
    } else {
        None
    };

    let _tray_handle = if use_xembed_tray {
        None
    } else {
        match crate::tray_icon::start(
            tray_command_sender,
            crate::tray_icon::TrayStatus {
                screensaver_enabled:
                    cfg.screensaver_enabled,
                windowshader_enabled,
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


            "playlist" => {

                let interval_source =
                    cfg.mode
                        .split(':')
                        .nth(2)
                        .unwrap_or("60");


                let result =
                    crate::parse_interval::parse_interval(
                        interval_source
                    );


                if cfg.debug_log {

                    crate::logger::debug(
                        &logfile,
                        "[MAIN] === PLAYLIST INTERVAL PARSE ===",
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

    if cfg.screen_lock_enabled && !lxde_x11_lock_unsupported {
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
    } else if !cfg.screen_lock_enabled {
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


    // Each presentation target owns an independent runtime configuration.
    // Keep the existing mutually exclusive supervisor startup until the
    // screensaver/lock-screen/Control Center handoff paths can coordinate both.
    let wallpaper_runtime =
        crate::define_wallpaper::WallpaperRuntime {
            display_format:
                crate::manage_configuration::WallpaperDisplayFormat::FullScreen,
            monitor_mode:
                cfg.wallpaper.monitor_mode,
            notifications:
                cfg.wallpaper.notifications,
            lyrics_enabled:
                cfg.lyrics_enabled,
            lyrics_state:
                None,
            lyrics_seek_handle:
                None,
            lyrics_navigation_handle:
                None,
            texture_policy:
                cfg.wallpaper_texture_policy,
            fps_policy:
                cfg.wallpaper_fps_policy,
            animation_speed_policy:
                cfg.wallpaper_speed_policy,
            postprocess_policy:
                cfg.wallpaper_postprocess_policy,
            audio_bands:
                audio_bands.clone(),
            tray_status:
                tray_status.clone(),
        };


    let windowshader_runtime =
        crate::define_wallpaper::WallpaperRuntime {
            display_format:
                crate::manage_configuration::WallpaperDisplayFormat::Windowed,
            monitor_mode:
                cfg.wallpaper.monitor_mode,
            notifications:
                cfg.wallpaper.notifications,
            lyrics_enabled:
                cfg.lyrics_enabled,
            lyrics_state:
                None,
            lyrics_seek_handle:
                None,
            lyrics_navigation_handle:
                None,
            texture_policy:
                cfg.windowshader_texture_policy.clone(),
            fps_policy:
                cfg.windowshader_fps_policy.clone(),
            animation_speed_policy:
                cfg.windowshader_speed_policy.clone(),
            postprocess_policy:
                cfg.windowshader_postprocess_policy.clone(),
            audio_bands:
                audio_bands.clone(),
            tray_status:
                tray_status.clone(),
        };

    // Each enabled target owns its own supervisor thread and renderer backend.
    // Both share the session shutdown flag, but not presentation pause state.
    let mut wallpaper_manager = if wallpaper_enabled_this_launch {
        Some(crate::manage_wallpaper_runtime::WallpaperRuntimeManager::start(
            wallpaper_enabled_this_launch, cfg.wallpaper_mode.clone(),
            wallpaper_runtime.clone(), logfile.clone(), Arc::clone(&running)))
    } else { None };
    let mut windowshader_manager = if windowshader_enabled {
        Some(crate::manage_windowshader_runtime::WindowshaderRuntimeManager::start(
            windowshader_enabled, cfg.windowshader_mode.clone(),
            windowshader_runtime, logfile.clone(), Arc::clone(&running)))
    } else { None };
    // The coordinator owns cloned handles, never supervisor threads.
    let wallpaper_control = crate::manage_wallpaper_runtime::PresentationRuntimeControl::new(
        wallpaper_manager.as_ref().map(|manager| manager.control()),
        windowshader_manager.as_ref().map(|manager| manager.control()),
    );


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


    let mut screensaver_waiting_for_active = false;

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


            Ok(command @ (crate::tray_icon::TrayCommand::Edit | crate::tray_icon::TrayCommand::EditWallpaper | crate::tray_icon::TrayCommand::EditWindowshader)) => {

                println!(
                    "[TRAY] Control Center requested."
                );

                crate::logger::information(
                    &logfile,
                    "[TRAY] Control Center requested",
                );

                let active_policy = match command {
                    crate::tray_icon::TrayCommand::EditWallpaper => tray_status.active_wallpaper(),
                    crate::tray_icon::TrayCommand::EditWindowshader => tray_status.active_windowshader(),
                    _ => None,
                };

                // Always coordinate both renderers, even when tray metadata
                // does not identify an active Wallpaper policy.
                let wallpaper_pause_acquired =
                    wallpaper_control.request_pause_after_first_frame(
                        running.as_ref()
                    );

                let edit_result =
                    if !wallpaper_pause_acquired {
                        Err(
                            "Timed out waiting for the active wallpaper renderer to acknowledge suspension; Control Center was not opened."
                                .to_string()
                        )
                    } else {
                        match (command, active_policy) {
                            (crate::tray_icon::TrayCommand::Edit, _) => {
                                crate::edit_shader::run_empty_session_prepaused(audio_bands.clone())
                            }
                            (crate::tray_icon::TrayCommand::EditWallpaper, Some(active)) => {
                                crate::edit_shader::run_wallpaper_only_prepaused(
                                    active.path, active.policy_id, audio_bands.clone(),
                                )
                            }
                            (crate::tray_icon::TrayCommand::EditWindowshader, Some(active)) => {
                                crate::edit_shader::run_windowshader_only_prepaused(
                                    active.path, active.policy_id, audio_bands.clone(),
                                )
                            }
                            _ => Err("The requested renderer has no active policy to edit.".to_string()),
                        }
                    };

                if edit_result.is_ok() {
                    let config_path =
                        crate::locate_paths::config_path();

                    match crate::load_config::load_config(
                        &config_path
                    ) {
                        Ok(config_result) => {
                            let reloaded_debug_log =
                                config_result.config.debug_log;

                            let reloaded_log_level =
                                config_result.config.log_level;

                            crate::logger::set_enabled(
                                reloaded_debug_log
                            );

                            crate::logger::set_log_level(
                                reloaded_log_level
                            );

                            if reloaded_debug_log {
                                for diagnostic in
                                    &config_result.diagnostics
                                {
                                    crate::logger::debug(
                                        &logfile,
                                        diagnostic,
                                    );
                                }
                            }

                            cfg = config_result.config;

                            wallpaper_control.request_policy_reload(&cfg);

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

                // Release the coordinator even if the tray has no active-wallpaper
                // metadata: Windowshader can be active independently.
                if wallpaper_pause_acquired && !wallpaper_control.resume_and_wait_for_frame(running.as_ref()) {
                    crate::logger::warning(&logfile, "[TRAY] Presentation resume was not confirmed by every active target");
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


        // An SDL renderer exit does not itself reset the compositor's idle
        // notification. Require a fresh Active observation before rearming.
        if screensaver_waiting_for_active {
            match session_state {
                crate::query_session::SessionState::Active => {
                    screensaver_waiting_for_active = false;
                    crate::logger::information(
                        &logfile,
                        "[SESSION REARM] Active observed; idle activation rearmed",
                    );
                }
                crate::query_session::SessionState::Idle => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
            }
        }

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
                        "playlist" => {
                            let playlist_id =
                                parsed_mode
                                    .argument
                                    .trim()
                                    .parse::<i64>()
                                    .ok()
                                    .filter(|playlist_id| *playlist_id > 0)
                                    .unwrap_or(0);

                            crate::manage_shader::ShaderMode::Playlist(
                                playlist_id
                            )
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

                    if cfg.screen_lock_enabled && !lxde_x11_lock_unsupported {
                        crate::logger::information(
                            &logfile,
                            "[LOCK] Screensaver idle threshold reached; engaging negotiated secure-lock backend",
                        );

                        println!(
                            "[MAIN] Screensaver idle threshold reached: engaging negotiated secure-lock backend"
                        );


                        // Prefer supported desktop-native lockers whenever
                        // one is available. KDE, Xfce, and GNOME retain ownership
                        // of secure locking and authentication; Screenshaver supplies
                        // shader presentation only. Other/unknown desktops fall
                        // through to the generic Wayland backend, which validates
                        // its required Wayland/ext-session-lock capabilities before
                        // acquiring a compositor-enforced lock and, by design, owns
                        // its own PAM authentication and authenticated unlock path.
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
                                                audio_bands.clone(),
                                                cfg.subtitles,
                                                cfg.subtitle_placement,
                                            )
                                        }

                                        _ => {
                                            Err(
                                                "GNOME shader lock presentation is unavailable because its runtime ownership or extension activation guard is missing"
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
                                    audio_bands.clone(),
                                    cfg.subtitles,
                                    cfg.subtitle_placement,
                                )
                            };


                        match lock_result {
                            Ok(()) => {
                                crate::logger::information(
                                    &logfile,
                                    "[LOCK] Secure-lock session completed",
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
                            audio_bands.clone(),
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

                            // Wait for the idle backend to report actual
                            // activity before allowing another activation.
                            screensaver_waiting_for_active = true;
                            crate::logger::information(
                                &logfile,
                                "[SESSION REARM] Screensaver exited; waiting for Active",
                            );
                            break 'screensaver_session;
                        }

                        crate::render_frame::ScreensaverRunOutcome::EditCurrentShader {
                            shader_path,
                            policy_id,
                        } => {
                            crate::logger::information(
                                &logfile,
                                &format!(
                                    "[SESSION] Editing active screensaver policy_id={} shader: {}",
                                    policy_id,
                                    shader_path.display(),
                                ),
                            );

                            let edit_result =
                                crate::edit_shader::run_screensaver_only(
                                    shader_path.clone(),
                                    policy_id,
                                    audio_bands.clone(),
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
                                    let reloaded_debug_log =
                                        config_result.config.debug_log;

                                    let reloaded_log_level =
                                        config_result.config.log_level;

                                    crate::logger::set_enabled(
                                        reloaded_debug_log
                                    );

                                    crate::logger::set_log_level(
                                        reloaded_log_level
                                    );

                                    if reloaded_debug_log {
                                        for diagnostic in
                                            &config_result.diagnostics
                                        {
                                            crate::logger::debug(
                                                &logfile,
                                                diagnostic,
                                            );
                                        }
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


    if let Some(manager) = windowshader_manager.as_mut() { manager.stop_and_join(); }
    if let Some(manager) = wallpaper_manager.as_mut() { manager.stop_and_join(); }


    if let Some(mut integration) =
        gnome_extension_integration.take()
    {
        match integration.deactivate() {
            Ok(()) => {
                crate::logger::information(
                    &logfile,
                    "[GNOME] Runtime extension guard released; Shell extension remains active",
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

        drop(_x11_tray_handle);
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

