#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Run,

    Start,

    Stop,

    Help,

    Version,

    Control {
        shader_name: Option<String>,
    },

    ConstructLockScreenKde,

    ConstructLockScreenXfce,
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
         Temporary development/setup options:\n\
         \n\
             --construct-lock-screen-kde OUTPUT_PATH\n\
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
