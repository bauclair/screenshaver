#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run,

    Help,

    Version,

    Reserved {
        option: String,
    },

    PreviewTexture {
        family: String,
        palette: Option<String>,
    },

    PreviewShader {
        shader_name: String,
        shader_texture: Option<String>,
        shader_palette: Option<String>,
    },

    ListTextures,

    ListPalettes,

    DeleteCache,
}


const VALID_TEXTURE_FAMILIES: [&str; 11] = [
    "random",
    "julia",
    "marble",
    "clouds",
    "cellular",
    "minerals",
    "mesh",
    "radial",
    "jigsaw",
    "noise",
    "bricks",
];


const VALID_PREVIEW_TEXTURE_FAMILIES: [&str; 10] = [
    "julia",
    "marble",
    "clouds",
    "cellular",
    "minerals",
    "mesh",
    "radial",
    "jigsaw",
    "noise",
    "bricks",
];


const VALID_TEXTURE_PALETTES: [&str; 7] = [
    "random",
    "slate",
    "sandstone",
    "lichen",
    "mist",
    "bronze",
    "brick",
];


const VALID_PREVIEW_TEXTURE_PALETTES: [&str; 6] = [
    "slate",
    "sandstone",
    "lichen",
    "mist",
    "bronze",
    "brick",
];


const RESERVED_OPTIONS: [&str; 9] = [
    "--diagnostics",
    "--list-backends",
    "--list-shaders",
    "--verify-cache",
    "--rebuild-cache",
    "--clean-cache",
    "--benchmark",
    "--evaluate",
    "--convert",
];


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

        "-h" | "--help" => {

            require_no_extra_arguments(
                &args,
                args[0].as_str(),
            )?;


            Ok(
                Command::Help
            )
        }


        "-V" | "--version" => {

            require_no_extra_arguments(
                &args,
                args[0].as_str(),
            )?;


            Ok(
                Command::Version
            )
        }


        "--preview-texture" => {

            parse_preview_texture(
                &args[1..]
            )
        }


        "--preview-shader" => {

            parse_preview_shader(
                &args[1..]
            )
        }


        "--list-textures" => {

            require_no_extra_arguments(
                &args,
                "--list-textures",
            )?;


            Ok(
                Command::ListTextures
            )
        }


        "--list-palettes" => {

            require_no_extra_arguments(
                &args,
                "--list-palettes",
            )?;


            Ok(
                Command::ListPalettes
            )
        }


        "--delete-cache" => {

            require_no_extra_arguments(
                &args,
                "--delete-cache",
            )?;


            Ok(
                Command::DeleteCache
            )
        }


        option
            if RESERVED_OPTIONS.contains(
                &option
            ) =>
        {

            require_no_extra_arguments(
                &args,
                option,
            )?;


            Ok(
                Command::Reserved {
                    option:
                        option.to_string(),
                }
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


fn parse_preview_texture(
    args: &[String],
) -> Result<Command, String> {

    let mut family:
        Option<String> =
            None;


    let mut palette:
        Option<String> =
            None;


    let mut index =
        0;


    while index
        < args.len()
    {
        match args[index].as_str() {

            "--family" => {

                if family.is_some() {

                    return Err(
                        "--family may only be specified once"
                            .to_string()
                    );
                }


                let value =
                    argument_value(
                        args,
                        index,
                        "--family",
                    )?;


                validate_preview_texture_family(
                    value
                )?;


                family =
                    Some(
                        value.to_ascii_lowercase()
                    );


                index += 2;
            }


            "--palette" => {

                if palette.is_some() {

                    return Err(
                        "--palette may only be specified once"
                            .to_string()
                    );
                }


                let value =
                    argument_value(
                        args,
                        index,
                        "--palette",
                    )?;


                validate_preview_texture_palette(
                    value
                )?;


                palette =
                    Some(
                        value.to_ascii_lowercase()
                    );


                index += 2;
            }


            "--background-texture" => {

                return Err(
                    "--background-texture is not valid with --preview-texture; use --family"
                        .to_string()
                );
            }


            option
                if option.starts_with('-') =>
            {

                return Err(
                    format!(
                        "Unknown option for --preview-texture: {}",
                        option
                    )
                );
            }


            argument => {

                return Err(
                    format!(
                        "Unexpected argument for --preview-texture: {}",
                        argument
                    )
                );
            }
        }
    }


    let family =
        family.ok_or_else(
            || {
                "--family is required with --preview-texture"
                    .to_string()
            }
        )?;


    Ok(
        Command::PreviewTexture {
            family,
            palette,
        }
    )
}


fn parse_preview_shader(
    args: &[String],
) -> Result<Command, String> {

    let shader_name =
        args.first()
            .ok_or_else(
                || {
                    "--preview-shader requires a shader filename or path"
                        .to_string()
                }
            )?;


    if shader_name.starts_with('-') {

        return Err(
            "--preview-shader requires a shader filename or path before its optional parameters"
                .to_string()
        );
    }


    let mut shader_texture:
        Option<String> =
            None;


    let mut shader_palette:
        Option<String> =
            None;


    let mut index =
        1;


    while index
        < args.len()
    {
        match args[index].as_str() {

            "--shader-texture"
            | "--texture"
            | "--background-texture" => {

                if shader_texture.is_some() {

                    return Err(
                        "A shader texture option may only be specified once"
                            .to_string()
                    );
                }


                let option_name =
                    args[index].as_str();


                let value =
                    argument_value(
                        args,
                        index,
                        option_name,
                    )?;


                validate_texture_family(
                    value
                )?;


                shader_texture =
                    Some(
                        value.to_ascii_lowercase()
                    );


                index += 2;
            }


            "--shader-palette"
            | "--palette" => {

                if shader_palette.is_some() {

                    return Err(
                        "A shader palette option may only be specified once"
                            .to_string()
                    );
                }


                let option_name =
                    args[index].as_str();


                let value =
                    argument_value(
                        args,
                        index,
                        option_name,
                    )?;


                validate_texture_palette(
                    value
                )?;


                shader_palette =
                    Some(
                        value.to_ascii_lowercase()
                    );


                index += 2;
            }


            "--family" => {

                return Err(
                    "--family is not valid with --preview-shader; use --shader-texture"
                        .to_string()
                );
            }


            option
                if option.starts_with('-') =>
            {

                return Err(
                    format!(
                        "Unknown option for --preview-shader: {}",
                        option
                    )
                );
            }


            argument => {

                return Err(
                    format!(
                        "Unexpected argument for --preview-shader: {}",
                        argument
                    )
                );
            }
        }
    }


    Ok(
        Command::PreviewShader {
            shader_name:
                shader_name.to_string(),

            shader_texture,

            shader_palette,
        }
    )
}


fn argument_value<'a>(
    args: &'a [String],
    option_index: usize,
    option_name: &str,
) -> Result<&'a str, String> {

    let value =
        args.get(
            option_index + 1
        )
        .ok_or_else(
            || {
                format!(
                    "{} requires a value",
                    option_name
                )
            }
        )?;


    if value.starts_with('-') {

        return Err(
            format!(
                "{} requires a value",
                option_name
            )
        );
    }


    Ok(
        value.as_str()
    )
}


fn validate_preview_texture_family(
    value: &str,
) -> Result<(), String> {

    validate_named_value(
        value,
        "texture family",
        VALID_PREVIEW_TEXTURE_FAMILIES
            .as_slice(),
    )
}


fn validate_preview_texture_palette(
    value: &str,
) -> Result<(), String> {

    validate_named_value(
        value,
        "texture palette",
        VALID_PREVIEW_TEXTURE_PALETTES
            .as_slice(),
    )
}


fn validate_texture_family(
    value: &str,
) -> Result<(), String> {

    validate_named_value(
        value,
        "background texture",
        VALID_TEXTURE_FAMILIES
            .as_slice(),
    )
}


fn validate_texture_palette(
    value: &str,
) -> Result<(), String> {

    validate_named_value(
        value,
        "texture palette",
        VALID_TEXTURE_PALETTES
            .as_slice(),
    )
}


fn validate_named_value(
    value: &str,
    description: &str,
    valid_values: &[&str],
) -> Result<(), String> {

    let normalized =
        value.to_ascii_lowercase();


    if valid_values.contains(
        &normalized.as_str()
    ) {

        return Ok(());
    }


    Err(
        format!(
            "Unknown {}: {}\n\nValid values:\n    {}",
            description,
            value,
            valid_values.join("\n    "),
        )
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
             -h, --help\n\
                 Display this help information.\n\
         \n\
             -V, --version\n\
                 Display the Screenshaver version.\n\
         \n\
             --preview-texture --family FAMILY [--palette PALETTE]\n\
                 Preview one procedurally generated texture.\n\
                 This command does not consult screenshaver.toml.\n\
         \n\
             --preview-shader SHADER [--shader-texture FAMILY] [--shader-palette PALETTE]\n\
                 Preview one shader immediately in a full-screen window.\n\
                 SHADER may be a path or a filename in the shader folder.\n\
                 Command-line values override shader-specific and global values.\n\
                 --texture and --palette are accepted as shorter aliases.\n\
         \n\
             --list-textures\n\
                 Display available procedural texture families.\n\
         \n\
             --list-palettes\n\
                 Display available procedural texture palettes.\n\
         \n\
         Texture families:\n\
             julia\n\
             marble\n\
             clouds\n\
             cellular\n\
             minerals\n\
             mesh\n\
             radial\n\
             jigsaw\n\
             noise\n\
             bricks\n\
         \n\
         Texture palettes:\n\
             slate\n\
             sandstone\n\
             lichen\n\
             mist\n\
             bronze\n\
             brick\n\
         \n\
         Examples:\n\
             screenshaver --preview-texture --family julia\n\
             screenshaver --preview-texture --family marble --palette sandstone\n\
             screenshaver --preview-shader \"Heartfelt.glsl\"\n\
             screenshaver --preview-shader \"Heartfelt.glsl\" --background-texture clouds\n\
             screenshaver --preview-shader \"Heartfelt.glsl\" --palette mist\n\
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
             --delete-cache\n\
                 Delete all Screenshaver cached/preprocessed shaders.\n\
         \n\
             --convert\n\
                 Convert a GLSL shader using Screenshaver preprocessing.\n\
         \n\
         Configuration:\n\
             ~/.config/screenshaver/\n\
             ~/.config/screenshaver/screenshaver.toml\n\
             ~/.config/screenshaver/shaders/\n\
             ~/.config/screenshaver/cache/\n\
             ~/.config/screenshaver/rejected/\n\
             ~/.config/screenshaver/screenshaver.log\n\
         \n\
         Project status:\n\
             Screenshaver is under active development.",
        env!(
            "CARGO_PKG_VERSION"
        )
    );
}


pub fn print_reserved_option(
    option: &str,
) {

    println!(
        "Screenshaver {}\n\n\
         The '{}' option has been reserved for a future version of Screenshaver.\n\n\
         This feature has not yet been implemented.\n\n\
         Run 'screenshaver --help' to view available and reserved options.",
        env!(
            "CARGO_PKG_VERSION"
        ),
        option,
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

