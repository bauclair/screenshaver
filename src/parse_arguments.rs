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

    DiscoverOpenRgb,

    AmbientRecovery,

    TestOpenrgb { apply: bool, animate: bool },

    AmbientLightingTest {
        openrgb_host: Option<std::net::Ipv4Addr>,
    },

    TestLocalization,

    AuditTranslation {
        locale: Option<String>,
        module: Option<String>,
        all_locales: bool,
        online: bool,
    },

    TestSchemaReader {
        database_path: Option<String>,
    },

    TestSchemaReconstruction {
        source_path: String,
        destination_path: String,
    },

    TestSchemaMigrationFailures {
        fixture_path: String,
        work_directory: String,
    },

    TestDatabaseCutoverRecovery,

    TestMigrationCoordinator {
        database_path: Option<String>,
    },

    BenchmarkRender {
        shader_path: String,
    },

    TestAudioMotion {
        shader_path: String,
    },

    TestReverseAnimation {
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


        "--test-openrgb" => {
            match args.as_slice() {
                [_] => Ok(Command::TestOpenrgb { apply: false, animate: false }),
                [_, flag] if flag == "--apply" => Ok(Command::TestOpenrgb { apply: true, animate: false }),
                [_, flag] if flag == "--animate" => Ok(Command::TestOpenrgb { apply: false, animate: true }),
                [_, first, second] if (first == "--animate" && second == "--apply") || (first == "--apply" && second == "--animate") => Ok(Command::TestOpenrgb { apply: true, animate: true }),
                _ => Err("Usage: --test-openrgb [--animate] [--apply]".into()),
            }
        }

        "--ambient-recovery" => {
            require_no_extra_arguments(&args, "--ambient-recovery")?;
            Ok(Command::AmbientRecovery)
        }

        "--discover-openrgb" => {
            require_no_extra_arguments(&args, "--discover-openrgb")?;
            Ok(Command::DiscoverOpenRgb)
        }


        "--ambient-lighting-test" => {
            let openrgb_host = match args.as_slice() {
                [_] => None,
                [_, flag, value] if flag == "--openrgb-host" => {
                    Some(value.parse::<std::net::Ipv4Addr>()
                        .map_err(|_| "--openrgb-host requires a valid IPv4 address".to_string())?)
                }
                _ => return Err("Usage: --ambient-lighting-test [--openrgb-host VM_IPV4_ADDRESS]".into()),
            };
            Ok(Command::AmbientLightingTest { openrgb_host })
        }


        "--test-localization" => {

            require_no_extra_arguments(
                &args,
                "--test-localization",
            )?;


            Ok(
                Command::TestLocalization
            )
        }


        "--audit-translation" => {

            parse_audit_translation(
                &args[1..]
            )
        }


        "--test-schema-reader" => {

            parse_test_schema_reader(
                &args[1..]
            )
        }


        "--test-schema-reconstruction" => {

            parse_test_schema_reconstruction(
                &args[1..]
            )
        }


        "--test-schema-migration-failures" => {

            parse_test_schema_migration_failures(
                &args[1..]
            )
        }


        "--test-database-cutover-recovery" => {

            require_no_extra_arguments(
                &args,
                "--test-database-cutover-recovery",
            )?;


            Ok(
                Command::TestDatabaseCutoverRecovery
            )
        }


        "--test-migration-coordinator" => {

            parse_test_migration_coordinator(
                &args[1..]
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


        "--test-reverse-animation" => {

            parse_test_reverse_animation(
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


fn parse_audit_translation(
    args: &[String],
) -> Result<Command, String> {
    let mut online = false;
    let mut positional = Vec::new();

    for argument in args {
        let value = argument.trim();
        if value == "--online" {
            if online {
                return Err("--audit-translation accepts --online only once".to_string());
            }
            online = true;
        } else {
            positional.push(value);
        }
    }

    if positional.len() > 2 {
        return Err(
            "--audit-translation accepts at most a locale (or --all), a module, and optional --online"
                .to_string()
        );
    }

    let all_locales = positional.first().map(|value| *value == "--all").unwrap_or(false);

    if online && all_locales {
        return Err(
            "--audit-translation cannot use --online together with --all; online evaluation must be run for one locale at a time"
                .to_string()
        );
    }

    let locale = if all_locales {
        None
    } else {
        match positional.first() {
            Some(value) => {
                if value.is_empty() || value.starts_with('-') {
                    return Err(
                        "--audit-translation accepts an optional locale such as es-US, or --all, followed by an optional module and --online"
                            .to_string()
                    );
                }
                Some((*value).to_string())
            }
            None => None,
        }
    };

    let module = match positional.get(1) {
        Some(value) => {
            if value.is_empty() || value.starts_with('-') {
                return Err(
                    "--audit-translation MODULE must name a Rust source module such as import_data"
                        .to_string()
                );
            }
            Some((*value).to_string())
        }
        None => None,
    };

    Ok(Command::AuditTranslation {
        locale,
        module,
        all_locales,
        online,
    })
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


fn parse_test_schema_reader(
    args: &[String],
) -> Result<Command, String> {

    if args.len() > 1 {
        return Err(
            "--test-schema-reader accepts at most one database path"
                .to_string()
        );
    }

    let database_path =
        match args.first() {
            Some(value) => {
                let value = value.trim();

                if value.is_empty() || value.starts_with('-') {
                    return Err(
                        "--test-schema-reader accepts an optional database path"
                            .to_string()
                    );
                }

                Some(value.to_string())
            }

            None => None,
        };

    Ok(
        Command::TestSchemaReader {
            database_path,
        }
    )
}


fn parse_test_schema_reconstruction(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 2 {
        return Err(
            "--test-schema-reconstruction requires exactly two database paths: SOURCE DESTINATION"
                .to_string()
        );
    }

    let source_path =
        args[0].trim();

    let destination_path =
        args[1].trim();

    if source_path.is_empty()
        || source_path.starts_with('-')
        || destination_path.is_empty()
        || destination_path.starts_with('-')
    {
        return Err(
            "--test-schema-reconstruction requires valid SOURCE and DESTINATION database paths"
                .to_string()
        );
    }

    Ok(
        Command::TestSchemaReconstruction {
            source_path:
                source_path.to_string(),

            destination_path:
                destination_path.to_string(),
        }
    )
}


fn parse_test_schema_migration_failures(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 2 {
        return Err(
            "--test-schema-migration-failures requires exactly two paths: FIXTURE WORK_DIRECTORY"
                .to_string()
        );
    }

    let fixture_path =
        args[0].trim();

    let work_directory =
        args[1].trim();

    if fixture_path.is_empty()
        || fixture_path.starts_with('-')
        || work_directory.is_empty()
        || work_directory.starts_with('-')
    {
        return Err(
            "--test-schema-migration-failures requires valid FIXTURE and WORK_DIRECTORY paths"
                .to_string()
        );
    }

    Ok(
        Command::TestSchemaMigrationFailures {
            fixture_path:
                fixture_path.to_string(),

            work_directory:
                work_directory.to_string(),
        }
    )
}


fn parse_test_migration_coordinator(
    args: &[String],
) -> Result<Command, String> {

    if args.len() > 1 {
        return Err(
            "--test-migration-coordinator accepts at most one Schema-1 database path"
                .to_string()
        );
    }

    let database_path =
        match args.first() {
            Some(value) => {
                let value = value.trim();

                if value.is_empty()
                    || value.starts_with('-')
                {
                    return Err(
                        "--test-migration-coordinator accepts an optional Schema-1 database path"
                            .to_string()
                    );
                }

                Some(
                    value.to_string()
                )
            }

            None => None,
        };

    Ok(
        Command::TestMigrationCoordinator {
            database_path,
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



fn parse_test_reverse_animation(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 1 {
        return Err(
            "--test-reverse-animation requires exactly one shader filename or path"
                .to_string()
        );
    }

    let shader_path =
        args[0].trim();

    if shader_path.is_empty()
        || shader_path.starts_with('-')
    {
        return Err(
            "--test-reverse-animation requires a valid shader filename or path"
                .to_string()
        );
    }

    Ok(
        Command::TestReverseAnimation {
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
             --test-reverse-animation <SHADER_PATH>\n\
                 Render one shader at exactly -1.0x animation speed for 60 seconds.\n\
                 Press Esc to stop the test and return to the command line.\n\
                 Test load failures do not change shader validation/rejection state.\n\
         \n\
             --test-playlists\n\
                 Run developer Playlist database-management tests and exit.\n\
         \n\
             --test-lyrics\n\
                 Run the synchronized-lyrics development test and exit.\n\
         \n\
             --test-openrgb [--animate] [--apply]\n\
                 Guarded Cynosa lighting test (dry run by default).\n\
         \n\
             --ambient-recovery\n\
                 Read-only inspection of pending OpenRGB recovery records; never changes lighting.\n\
         \n\
             --discover-openrgb\n\
                 Read-only inventory of devices recognized by the local OpenRGB SDK server.\n\
         \n\
             --ambient-lighting-test [--openrgb-host VM_IPV4_ADDRESS]\n\
                 Run the software-only ambient-lighting test.\n\
                 --openrgb-host connects to an explicitly specified OpenRGB VM on TCP port 16743.\n\
         \n\
             --test-localization\n\
                 Test locale selection, translation lookup, and English fallback.\n\
         \n\
             --audit-translation [LOCALE|--all] [MODULE]\n\
                 Audit Rust source for user-facing English text not covered by localization.\n\
                 If LOCALE is omitted, use [language].locale from screenshaver.toml.\n\
                 Use --all to audit every locale supported by the current Screenshaver build.\n\
                 If MODULE is supplied, scan only that Rust module while still auditing the selected locale catalog(s).\n\
         \n\
             --test-schema-reader [DATABASE_PATH]\n\
                 Read a Schema-1 database through the historical migration reader and exit.\n\
                 If DATABASE_PATH is omitted, use the normal Screenshaver database.\n\
                 Opens the selected database read-only and bypasses normal database preparation.\n\
         \n\
             --test-database-cutover-recovery\n\
             --test-migration-coordinator\n\
                  Exercise database migration cutover/recovery filesystem states and exit.\n\
                  Uses only a disposable temporary directory; the live database is not accessed.\n\
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
