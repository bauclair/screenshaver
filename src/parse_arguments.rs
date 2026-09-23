#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Run,

    Start,

    Stop,

    Help,

    Version,

    CompareDatabases {
        database_a: String,
        database_b: String,
        exclude_metadata: bool,
        exclude_local_config: bool,
    },

    TestPlaylists,

    TestLyrics,

    TestVocalDetection,

    BenchmarkRender {
        shader_path: String,
    },

    TestAudioMotion {
        shader_path: String,
    },

    Control {
        shader_name: Option<String>,
    },

    ConstructLockScreenKde,

    ConstructLockScreenXfce,

    ResetIdleTimeout {
        value: String,
    },
}




pub fn parse() -> Result<Command, String> {

    let args =
        std::env::args()
            .skip(1)
            .collect::<Vec<_>>();


    if args.is_empty() {

        return Ok(
            Command::Run
        );
    }


    match args[0].as_str() {

        "--start" => {

            require_no_extra_arguments(
                &args,
                "--start",
            )?;


            Ok(
                Command::Start
            )
        }


        "--stop" => {

            require_no_extra_arguments(
                &args,
                "--stop",
            )?;


            Ok(
                Command::Stop
            )
        }


        "-h"
        | "--help" => {

            require_no_extra_arguments(
                &args,
                args[0].as_str(),
            )?;


            Ok(
                Command::Help
            )
        }


        "-V"
        | "--version" => {

            require_no_extra_arguments(
                &args,
                args[0].as_str(),
            )?;


            Ok(
                Command::Version
            )
        }


        "--compare-databases" => {

            parse_compare_databases(
                &args[1..]
            )
        }


        "--test-playlists" => {

            require_no_extra_arguments(
                &args,
                "--test-playlists",
            )?;


            Ok(
                Command::TestPlaylists
            )
        }


        "--test-lyrics" => {

            require_no_extra_arguments(
                &args,
                "--test-lyrics",
            )?;


            Ok(
                Command::TestLyrics
            )
        }


        "--test-vocal-detection" => {

            require_no_extra_arguments(
                &args,
                "--test-vocal-detection",
            )?;


            Ok(
                Command::TestVocalDetection
            )
        }


        "--benchmark-render" => {

            parse_benchmark_render(
                &args[1..]
            )
        }


        "--test-audio-motion" => {

            parse_test_audio_motion(
                &args[1..]
            )
        }


        "--reset-idle-timeout" => {

            parse_reset_idle_timeout(
                &args[1..]
            )
        }


        "--control" => {

            parse_control(
                &args[1..]
            )
        }


        "--construct-lock-screen-kde" => {

            parse_construct_lock_screen_kde(
                &args[1..]
            )
        }


        "--construct-lock-screen-xfce" => {

            require_no_extra_arguments(
                &args,
                "--construct-lock-screen-xfce",
            )?;


            Ok(
                Command::ConstructLockScreenXfce
            )
        }


        option
            if option.starts_with('-') =>
        {

            Err(
                format!(
                    "Unknown option: {}",
                    option
                )
            )
        }


        argument => {

            Err(
                format!(
                    "Unexpected argument: {}",
                    argument
                )
            )
        }
    }
}


fn parse_compare_databases(
    args: &[String],
) -> Result<Command, String> {

    let mut exclude_metadata = false;
    let mut exclude_local_config = false;
    let mut database_paths = Vec::new();

    for argument in args {
        if argument == "--exclude-metadata" {
            if exclude_metadata {
                return Err(
                    "--compare-databases accepts --exclude-metadata only once"
                        .to_string()
                );
            }

            exclude_metadata = true;
            continue;
        }

        if argument == "--exclude-local-config" {
            if exclude_local_config {
                return Err(
                    "--compare-databases accepts --exclude-local-config only once"
                        .to_string()
                );
            }

            exclude_local_config = true;
            continue;
        }

        if argument.starts_with('-') {
            return Err(
                format!(
                    "Unknown --compare-databases option: {}",
                    argument
                )
            );
        }

        let value = argument.trim();

        if value.is_empty() {
            return Err(
                "--compare-databases requires two valid database paths"
                    .to_string()
            );
        }

        database_paths.push(
            value.to_string()
        );
    }

    if database_paths.len() != 2 {
        return Err(
            "--compare-databases requires exactly two database paths, with optional --exclude-metadata and/or --exclude-local-config"
                .to_string()
        );
    }

    Ok(
        Command::CompareDatabases {
            database_a: database_paths[0].clone(),
            database_b: database_paths[1].clone(),
            exclude_metadata,
            exclude_local_config,
        }
    )
}


fn parse_benchmark_render(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 1 {
        return Err(
            "--benchmark-render requires exactly one shader filename or path"
                .to_string()
        );
    }

    let shader_path =
        args[0].trim();

    if shader_path.is_empty()
        || shader_path.starts_with('-')
    {
        return Err(
            "--benchmark-render requires a valid shader filename or path"
                .to_string()
        );
    }

    Ok(
        Command::BenchmarkRender {
            shader_path:
                shader_path.to_string(),
        }
    )
}


fn parse_test_audio_motion(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 1 {
        return Err(
            "--test-audio-motion requires exactly one shader filename or path"
                .to_string()
        );
    }

    let shader_path =
        args[0].trim();

    if shader_path.is_empty()
        || shader_path.starts_with('-')
    {
        return Err(
            "--test-audio-motion requires a valid shader filename or path"
                .to_string()
        );
    }

    Ok(
        Command::TestAudioMotion {
            shader_path:
                shader_path.to_string(),
        }
    )
}


fn parse_reset_idle_timeout(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 1 {
        return Err(
            "--reset-idle-timeout requires exactly one duration (for example: 60s, 2m, or 1h)"
                .to_string()
        );
    }

    let value = args[0].trim();

    if value.is_empty() || value.starts_with('-') {
        return Err(
            "--reset-idle-timeout requires a positive duration (for example: 60s, 2m, or 1h)"
                .to_string()
        );
    }

    Ok(
        Command::ResetIdleTimeout {
            value: value.to_string(),
        }
    )
}


fn parse_control(
    args: &[String],
) -> Result<Command, String> {

    if args.len() > 1 {

        return Err(
            "--control accepts at most one shader filename or path"
                .to_string()
        );
    }


    let shader_name =
        match args.first() {

            Some(value) => {

                let value =
                    value.trim();


                if value.is_empty()
                    || value.starts_with('-')
                {

                    return Err(
                        "--control accepts an optional shader filename or path"
                            .to_string()
                    );
                }


                Some(
                    value.to_string()
                )
            }


            None => {
                None
            }
        };


    Ok(
        Command::Control {
            shader_name,
        }
    )
}


fn parse_construct_lock_screen_kde(
    args: &[String],
) -> Result<Command, String> {

    if !args.is_empty() {

        return Err(
            "--construct-lock-screen-kde does not accept additional arguments"
                .to_string()
        );
    }


    Ok(
        Command::ConstructLockScreenKde
    )
}

fn require_no_extra_arguments(
    args: &[String],
    option: &str,
) -> Result<(), String> {

    if args.len()
        == 1
    {

        return Ok(());
    }


    Err(
        format!(
            "{} does not accept additional arguments",
            option
        )
    )
}


pub fn print_version() {

    println!(
        "Screenshaver {}",
        env!(
            "CARGO_PKG_VERSION"
        )
    );
}


pub fn print_help() {

    println!(
        "Screenshaver {}\n\
         A modern cross-desktop screensaver for Linux.\n\
         \n\
         Usage:\n\
             screenshaver [OPTION]\n\
         \n\
         Available options:\n\
         \n\
             --start\n\
                 Start Screenshaver normally. Equivalent to launching without an option.\n\
         \n\
             --stop\n\
                 Stop the running Screenshaver program, regardless of its current state.\n\
         \n\
             -h, --help\n\
                 Display this help information.\n\
         \n\
             -V, --version\n\
                 Display the Screenshaver version.\n\
         \n\
             --control [PATH]\n\
                 Open the Screenshaver Control Center.\n\
                 If PATH is supplied, preload that shader for policy editing.\n\
         \n\
             --reset-idle-timeout <DURATION>\n\
                 Reset the database-backed screensaver idle timeout and exit.\n\
                 Examples: 60s, 2m, 1h. When screen locking is enabled,\n\
                 values below 60 seconds are stored as 60 seconds.\n\
         \n\
             --compare-databases <DATABASE_A> <DATABASE_B> [--exclude-metadata] [--exclude-local-config]\n\
                 Perform a comprehensive, read-only comparison of two Screenshaver databases.\n\
                 --exclude-metadata compares portable semantic data without local IDs/timestamps.\n\
                 --exclude-local-config omits runtime targets and application/target defaults.\n\
                 The two exclusion options may be used independently or together.\n\
         \n\
             --benchmark-render <SHADER_PATH>\n\
                 Benchmark a shader using Screenshaver rendering-path variants.\n\
                 Reports FPS, low-FPS, and frame-time statistics for comparison.\n\
         \n\
         Temporary development/setup options:\n\
         \n\
             --test-audio-motion <SHADER_PATH>\n\
                  Run the experimental audio-motion shader harness.\n\
                  Press Esc or close the test window to exit.\n\
         \n\
             --test-playlists\n\
                 Run developer Playlist database-management tests and exit.\n\
         \n\
             --test-lyrics\n\
                 Run the synchronized-lyrics development test and exit.\n\
         \n\
             --test-vocal-detection\n\
                 Load and inspect the experimental singing-voice ONNX model, then exit.\n\
         \n\
             --construct-lock-screen-kde\n\
                 Construct/install the KDE lock-screen integration.\n\
                 Temporary development/setup command.\n\
         \n\
             --construct-lock-screen-xfce\n\
                 Construct/configure the Xfce lock-screen integration.\n\
                 Temporary development/setup command.\n\
         \n\
         Examples:\n\
             screenshaver --start\n\
             screenshaver --stop\n\
             screenshaver --control\n\
             screenshaver --control \"Heartfelt.glsl\"\n\
         \n\
         Configuration:\n\
             ~/.config/screenshaver/\n\
             ~/.config/screenshaver/screenshaver.toml\n\
             ~/.config/screenshaver/shaders/\n\
             ~/.config/screenshaver/screenshaver.log\n\
         \n\
         Project status:\n\
             Screenshaver is under active development.",
        env!(
            "CARGO_PKG_VERSION"
        )
    );
}


pub fn print_error(
    error: &str,
) {

    eprintln!(
        "Screenshaver {}\n\n\
         {}\n\n\
         Run 'screenshaver --help' to view available options.",
        env!(
            "CARGO_PKG_VERSION"
        ),
        error,
    );
}
