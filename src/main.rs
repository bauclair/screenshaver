mod load_config;
mod singleton;
mod logger;
mod parse_mode;
mod parse_interval;
mod parse_duration;
mod parse_arguments;
mod parse_subtitle_placement;

mod define_constants;
mod locate_paths;
mod delete_cache;

mod query_session;
mod session_backend;

mod manage_shader;
mod manage_textures;
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
mod generate_jigsaw;
mod generate_marble;
mod generate_mesh;
mod generate_minerals;
mod generate_noise;
mod generate_radial;
mod generate_textures;
mod preview_texture;
mod preview_shader;
mod preview_shader_directory;
mod palettes;
mod display_texture;
mod construct_text_overlay;
mod display_overlay;

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

match command {

    crate::parse_arguments::Command::Run => {}


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
        family,
        palette,
    } => {

        crate::preview_texture::run(
            family,
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
} => {

    match crate::preview_shader::run(
        shader_name,
        shader_texture,
        shader_palette,
        interval_seconds,
        fps,
    ) {

        Ok(()) => {}

        Err(error) => {

            eprintln!(
                "[SHADER PREVIEW] {}",
                error
            );
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
        }
    }


    return;
}


crate::parse_arguments::Command::ListTextures => {

    println!(
        "Available texture families:"
    );

    println!(
        "    julia"
    );

    println!(
        "    marble"
    );

    println!(
        "    clouds"
    );

    println!(
        "    cellular"
    );

    println!(
        "    minerals"
    );

    println!(
        "    mesh"
    );

    println!(
        "    radial"
    );

    println!(
        "    jigsaw"
    ); 

    println!(
        "    noise"
    ); 

    println!(
        "    bricks"
    ); 

    return;
}


crate::parse_arguments::Command::ListPalettes => {

    println!(
        "Available texture palettes:"
    );

    println!(
        "    slate"
    );

    println!(
        "    sandstone"
    );

    println!(
        "    lichen"
    );

    println!(
        "    mist"
    );

    println!(
        "    bronze"
    );

    println!(
        "    brick"
    );

    return;
}

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

                return;
            }
        };


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

                return;
            }
        };


    let cfg =
        result.config;


    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::create_log(
        &logfile
    );


    if cfg.debug_log {

        crate::logger::log(
            &logfile,
            "[MAIN] === CONFIG DUMP ===",
        );


        for line in &result.diagnostics {

            crate::logger::log(
                &logfile,
                line,
            );
        }


        crate::logger::log(
            &logfile,
            "[MAIN] === CONFIG END ===",
        );
    }


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

        crate::logger::log(
            &logfile,
            "[MAIN] === MODE PARSE ===",
        );


        for line in &parsed_mode.diagnostics {

            crate::logger::log(
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

                    crate::logger::log(
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

                    crate::logger::log(
                        &logfile,
                        "[MAIN] === INTERVAL PARSE ===",
                    );


                    for line in &result.diagnostics {

                        crate::logger::log(
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

        crate::logger::log(
            &logfile,
            "[MAIN] === IDLE PARSE ===",
        );


        for line in &parsed_idle.diagnostics {

            crate::logger::log(
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


                if cfg.debug_log {

                    crate::logger::log(
                        &logfile,
                        &format!(
                            "[MAIN] SESSION ERROR: {}",
                            error
                        ),
                    );
                }


                return;
            }
        };


    println!(
        "[MAIN] Session backend = {}",
        session.backend_name()
    );


    if cfg.debug_log {

        crate::logger::log(
            &logfile,
            &format!(
                "[MAIN] Session backend = {}",
                session.backend_name()
            ),
        );
    }


    let sdl =
        match sdl2::init() {

            Ok(sdl) => sdl,

            Err(error) => {

                eprintln!(
                    "[MAIN] SDL initialization failed: {}",
                    error
                );


                if cfg.debug_log {

                    crate::logger::log(
                        &logfile,
                        &format!(
                            "[MAIN] SDL initialization failed: {}",
                            error
                        ),
                    );
                }


                return;
            }
        };


    if cfg.show_splash {

        println!(
            "[MAIN] Displaying splash screen..."
        );


        if cfg.debug_log {

            crate::logger::log(
                &logfile,
                "[SPLASH] Displaying splash screen",
            );
        }


        match crate::splash_screen::show_splash(
            &sdl
        ) {

            Ok(()) => {

                if cfg.debug_log {

                    crate::logger::log(
                        &logfile,
                        "[SPLASH] Splash screen complete",
                    );
                }
            }

            Err(error) => {

                eprintln!(
                    "[MAIN] Splash screen failed: {}",
                    error
                );


                if cfg.debug_log {

                    crate::logger::log(
                        &logfile,
                        &format!(
                            "[SPLASH] Splash screen failed: {}",
                            error
                        ),
                    );
                }
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


    if cfg.debug_log {

        crate::logger::log(
            &logfile,
            "[MAIN] === ENTERING SESSION LOOP ===",
        );
    }


    println!(
        "[MAIN] Entering session loop..."
    );


    while running.load(Ordering::SeqCst) {

        let session_state =
            match session.poll_state() {

                Ok(state) => state,

                Err(error) => {

                    eprintln!(
                        "[MAIN] SESSION QUERY ERROR: {}",
                        error
                    );


                    if cfg.debug_log {

                        crate::logger::log(
                            &logfile,
                            &format!(
                                "[MAIN] SESSION QUERY ERROR: {}",
                                error
                            ),
                        );
                    }


                    break;
                }
            };


        match session_state {

            crate::query_session::SessionState::Idle => {

                if cfg.debug_log {

                    crate::logger::log(
                        &logfile,
                        "[SESSION] Session idle: engaging renderer",
                    );
                }


                println!(
                    "[MAIN] Session idle: engaging renderer"
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


                let shader_manager =
                    crate::manage_shader::ShaderManager::new(
                        shader_mode
                    );


                let mut renderer =
                    match crate::render_frame::FrameRenderer::new(
                        &sdl,
                        shader_manager,
                        parsed_interval.seconds,
                        cfg.global_rendered_fps,
                        cfg.fps_overrides.clone(),
                        cfg.texture_policy.clone(),
                        cfg.subtitles,
                        cfg.subtitle_placement,
                    ) {

                        Ok(renderer) => renderer,

                        Err(error) => {

                            eprintln!(
                                "[MAIN] Renderer initialization failed: {}",
                                error
                            );


                            if cfg.debug_log {

                                crate::logger::log(
                                    &logfile,
                                    &format!(
                                        "[MAIN] Renderer initialization failed: {}",
                                        error
                                    ),
                                );
                            }


                            break;
                        }
                    };


                if cfg.debug_log {

                    crate::logger::log(
                        &logfile,
                        "[RENDER] Renderer started",
                    );
                }


                renderer.run(
                    running.as_ref()
                );


                if running.load(Ordering::SeqCst) {

                    if cfg.debug_log {

                        crate::logger::log(
                            &logfile,
                            "[SESSION] User input: disengaging renderer",
                        );
                    }


                    println!(
                        "[MAIN] User input: disengaging renderer"
                    );
                }


                drop(renderer);


                if running.load(Ordering::SeqCst) {

                    if cfg.debug_log {

                        crate::logger::log(
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


    println!(
        "[MAIN] Pipeline complete."
    );
}

