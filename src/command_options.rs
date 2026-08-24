pub fn process_command_options() -> bool {
    let args: Vec<String> =
        std::env::args()
            .collect();

    if args.len() <= 1 {
        return false;
    }

    let option =
        args[1].as_str();

    match option {
        "-h" | "--help" => {
            show_help();
            true
        }

        "-V" | "--version" => {
            show_version();
            true
        }

        "--diagnostics"
        | "--list-backends"
        | "--list-shaders"
        | "--benchmark" => {
            show_reserved_option(option);
            true
        }

        _ => {
            show_unknown_option(option);
            true
        }
    }
}

fn show_version() {
    println!(
        "Screenshaver {}",
        env!("CARGO_PKG_VERSION")
    );
}

fn show_help() {
    println!(
        "Screenshaver {}\n\
         A modern cross-desktop screensaver for Linux.\n\
         \n\
         Usage:\n\
             screenshaver [OPTION]\n\
         \n\
         Available options:\n\
         \n\
             -h, --help\n\
                 Display this help information.\n\
         \n\
             -V, --version\n\
                 Display the Screenshaver version.\n\
         \n\
         Reserved options:\n\
         \n\
             --diagnostics\n\
                 Display system and desktop information useful for troubleshooting.\n\
         \n\
             --list-backends\n\
                 Display supported session backends.\n\
         \n\
             --list-shaders\n\
                 Display available shaders.\n\
         \n\
             --benchmark\n\
                 Benchmark shader loading and rendering performance.\n\
         \n\
         Configuration:\n\
             ~/.config/screenshaver/\n\
             ~/.config/screenshaver/screenshaver.toml\n\
             ~/.config/screenshaver/screensavers/\n\
             ~/.config/screenshaver/screenshaver.log\n\
         \n\
         Project status:\n\
             Screenshaver is under active development.",
        env!("CARGO_PKG_VERSION")
    );
}

fn show_reserved_option(
    option: &str,
) {
    println!(
        "Screenshaver {}\n\n\
         The '{}' option has been reserved for a future version of Screenshaver.\n\n\
         This feature has not yet been implemented.\n\n\
         Run 'screenshaver --help' to view available and reserved options.",
        env!("CARGO_PKG_VERSION"),
        option,
    );
}

fn show_unknown_option(
    option: &str,
) {
    println!(
        "Screenshaver {}\n\n\
         Unknown option: {}\n\n\
         Run 'screenshaver --help' to view available options.",
        env!("CARGO_PKG_VERSION"),
        option,
    );
}

