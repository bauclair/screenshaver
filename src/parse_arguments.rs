use crate::parse_texture_specification::{
    parse_texture_specification,
    TextureSpecification,
};

use crate::manage_overrides::{
    OverrideProperties,
    OverrideTarget,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Run,

    Start,


    Stop,

    Help,

    Version,

    Reserved {
        option: String,
    },

    PreviewTexture {
        texture: TextureSpecification,
        palette: Option<String>,
    },

    PreviewShader {
        shader_name: String,
        shader_texture: Option<TextureSpecification>,
        shader_palette: Option<String>,
        interval_seconds: Option<u64>,
        fps: Option<u32>,
        animation_speed: Option<f32>,
    },

    ListTextures,

    ListPalettes,

    DeleteCache,

    AddOverride {
        target: OverrideTarget,
        shader: String,
        properties: OverrideProperties,
    },

    DeleteOverride {
        target: OverrideTarget,
        shader: String,
    },

    ListOverrides {
        target: Option<OverrideTarget>,
    },
}


const VALID_TEXTURE_FAMILIES: [&str; 9] = [
    "random",
    "marble",
    "clouds",
    "cells",
    "mesh",
    "radial",
    "noise",
    "bricks",
    "hexagons",
];


const VALID_PREVIEW_TEXTURE_FAMILIES: [&str; 8] = [
    "marble",
    "clouds",
    "cells",
    "mesh",
    "radial",
    "noise",
    "bricks",
    "hexagons",
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


const RESERVED_OPTIONS: [&str; 5] = [
    "--verify-cache",
    "--rebuild-cache",
    "--clean-cache",
    "--benchmark",
    "--evaluate",
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


        "--add-override" => {

            parse_add_override(
                &args[1..]
            )
        }


        "--delete-override" => {

            parse_delete_override(
                &args[1..]
            )
        }


        "--list-overrides" => {

            parse_list_overrides(
                &args[1..]
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


fn parse_add_override(
    args: &[String],
) -> Result<Command, String> {

    if args.len() < 3 {
        return Err(
            "--add-override requires TARGET, SHADER, and at least one property"
                .to_string()
        );
    }

    let target =
        OverrideTarget::parse(
            &args[0]
        )?;

    let shader =
        parse_override_shader(
            &args[1],
            "--add-override",
        )?;

    let properties =
        parse_override_properties(
            &args[2..],
            target,
        )?;

    Ok(
        Command::AddOverride {
            target,
            shader,
            properties,
        }
    )
}


fn parse_delete_override(
    args: &[String],
) -> Result<Command, String> {

    if args.len() != 2 {
        return Err(
            "--delete-override requires exactly TARGET and SHADER"
                .to_string()
        );
    }

    let target =
        OverrideTarget::parse(
            &args[0]
        )?;

    let shader =
        parse_override_shader(
            &args[1],
            "--delete-override",
        )?;

    Ok(
        Command::DeleteOverride {
            target,
            shader,
        }
    )
}


fn parse_list_overrides(
    args: &[String],
) -> Result<Command, String> {

    if args.len() > 1 {
        return Err(
            "--list-overrides accepts at most one TARGET"
                .to_string()
        );
    }

    let target =
        args.first()
            .map(
                |value| OverrideTarget::parse(value)
            )
            .transpose()?;

    Ok(
        Command::ListOverrides {
            target,
        }
    )
}


fn parse_override_shader(
    value: &str,
    option: &str,
) -> Result<String, String> {

    let value =
        value.trim();

    if value.is_empty()
        || value.starts_with('-')
    {
        return Err(
            format!(
                "{} requires a shader filename",
                option,
            )
        );
    }

    Ok(
        value.to_string()
    )
}


fn parse_override_properties(
    args: &[String],
    target: OverrideTarget,
) -> Result<OverrideProperties, String> {

    let mut properties =
        OverrideProperties::default();

    for token in args {
        let (name, value) =
            split_override_property(
                token
            )?;

        match name
            .to_ascii_lowercase()
            .as_str()
        {
            "texture" => {
                if properties.texture.is_some() {
                    return Err(
                        "Override property 'texture' may only be specified once"
                            .to_string()
                    );
                }

                let family =
                    value.split(':')
                        .next()
                        .unwrap_or(value);

                validate_texture_family(
                    family
                )?;

                properties.texture =
                    Some(
                        value.to_ascii_lowercase()
                    );
            }

            "palette" => {
                if properties.palette.is_some() {
                    return Err(
                        "Override property 'palette' may only be specified once"
                            .to_string()
                    );
                }

                validate_texture_palette(
                    value
                )?;

                properties.palette =
                    Some(
                        value.to_ascii_lowercase()
                    );
            }

            "fps" => {
                if properties.fps.is_some() {
                    return Err(
                        "Override property 'fps' may only be specified once"
                            .to_string()
                    );
                }

                let fps =
                    value.parse::<u32>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid override FPS '{}'; specify an integer from {} through {}",
                                    value,
                                    crate::define_constants::MIN_RENDER_FPS,
                                    crate::define_constants::MAX_RENDER_FPS,
                                )
                            }
                        )?;

                if !(crate::define_constants::MIN_RENDER_FPS
                    ..=crate::define_constants::MAX_RENDER_FPS)
                    .contains(&fps)
                {
                    return Err(
                        format!(
                            "Override FPS {} is outside the supported range {}-{}",
                            fps,
                            crate::define_constants::MIN_RENDER_FPS,
                            crate::define_constants::MAX_RENDER_FPS,
                        )
                    );
                }

                properties.fps =
                    Some(fps);
            }

            "speed" => {
                if properties.speed.is_some() {
                    return Err(
                        "Override property 'speed' may only be specified once"
                            .to_string()
                    );
                }

                let speed =
                    value.parse::<f32>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid override speed '{}'; specify a numeric multiplier",
                                    value,
                                )
                            }
                        )?;

                let (minimum, maximum) =
                    match target {
                        OverrideTarget::Screensaver => (
                            crate::define_constants::SCREENSAVER_SPEED_MIN,
                            crate::define_constants::SCREENSAVER_SPEED_MAX,
                        ),

                        OverrideTarget::Wallpaper => (
                            crate::define_constants::WALLPAPER_SPEED_MIN,
                            crate::define_constants::WALLPAPER_SPEED_MAX,
                        ),
                    };

                if !speed.is_finite()
                    || !(minimum..=maximum)
                        .contains(&speed)
                {
                    return Err(
                        format!(
                            "Override speed {} for {} is outside the supported range {}-{}",
                            value,
                            target.name(),
                            minimum,
                            maximum,
                        )
                    );
                }

                properties.speed =
                    Some(speed);
            }

            other => {
                return Err(
                    format!(
                        "Unknown override property '{}'; supported properties: texture, palette, fps, speed",
                        other,
                    )
                );
            }
        }
    }

    if properties.is_empty() {
        return Err(
            "An override must define at least one property"
                .to_string()
        );
    }

    Ok(properties)
}


fn split_override_property(
    token: &str,
) -> Result<(&str, &str), String> {

    let pair =
        token.split_once('=')
            .or_else(
                || token.split_once(':')
            )
            .ok_or_else(
                || {
                    format!(
                        "Invalid override property '{}'; use NAME:VALUE or NAME=VALUE",
                        token,
                    )
                }
            )?;

    let name =
        pair.0.trim();

    let value =
        pair.1.trim();

    if name.is_empty()
        || value.is_empty()
    {
        return Err(
            format!(
                "Invalid override property '{}'; both name and value are required",
                token,
            )
        );
    }

    Ok((name, value))
}


fn parse_preview_texture(
    args: &[String],
) -> Result<Command, String> {

    let mut texture:
        Option<TextureSpecification> =
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

                if texture.is_some() {

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


                let parsed_texture =
                    parse_texture_specification(
                        value
                    )?;


                texture =
                    Some(
                        parsed_texture
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


            "--texture" => {

                return Err(
                    "--texture is not valid with --preview-texture; use --family"
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


    let texture =
        texture.ok_or_else(
            || {
                "--family is required with --preview-texture"
                    .to_string()
            }
        )?;


    Ok(
        Command::PreviewTexture {
            texture,
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
        Option<TextureSpecification> =
            None;


    let mut shader_palette:
        Option<String> =
            None;


    let mut interval_seconds:
        Option<u64> =
            None;


    let mut fps:
        Option<u32> =
            None;


    let mut animation_speed:
        Option<f32> =
            None;


    let mut index =
        1;


    while index
        < args.len()
    {
        match args[index].as_str() {

            | "--texture" => {

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


                shader_texture =
                    Some(
                        parse_texture_specification(
                            value
                        )?
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


            "--fps" => {

                if fps.is_some() {

                    return Err(
                        "--fps may only be specified once"
                            .to_string()
                    );
                }


                let value =
                    argument_value(
                        args,
                        index,
                        "--fps",
                    )?;


                let parsed_fps =
                    value.parse::<u32>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid --fps value '{}'; specify an integer from {} through {}",
                                    value,
                                    crate::define_constants::MIN_RENDER_FPS,
                                    crate::define_constants::MAX_RENDER_FPS,
                                )
                            }
                        )?;


                if !(
                    crate::define_constants::MIN_RENDER_FPS
                        ..=
                    crate::define_constants::MAX_RENDER_FPS
                )
                    .contains(
                        &parsed_fps
                    )
                {
                    return Err(
                        format!(
                            "--fps value {} is outside the supported range {}-{}",
                            parsed_fps,
                            crate::define_constants::MIN_RENDER_FPS,
                            crate::define_constants::MAX_RENDER_FPS,
                        )
                    );
                }


                fps =
                    Some(
                        parsed_fps
                    );


                index += 2;
            }


            "--speed" => {

                if animation_speed.is_some() {

                    return Err(
                        "--speed may only be specified once"
                            .to_string()
                    );
                }


                let value =
                    argument_value(
                        args,
                        index,
                        "--speed",
                    )?;


                let parsed_speed =
                    value.parse::<f32>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid --speed value '{}'; specify a number from {} through {}",
                                    value,
                                    crate::define_constants::PREVIEW_SPEED_MIN,
                                    crate::define_constants::PREVIEW_SPEED_MAX,
                                )
                            }
                        )?;


                if !parsed_speed.is_finite()
                    || !(
                        crate::define_constants::PREVIEW_SPEED_MIN
                            ..=
                        crate::define_constants::PREVIEW_SPEED_MAX
                    )
                        .contains(
                        &parsed_speed
                    )
                {
                    return Err(
                        format!(
                            "--speed value {} is outside the supported range {}-{}",
                            value,
                            crate::define_constants::PREVIEW_SPEED_MIN,
                            crate::define_constants::PREVIEW_SPEED_MAX,
                        )
                    );
                }


                animation_speed =
                    Some(
                        parsed_speed
                    );


                index += 2;
            }


            "--interval" => {

                if interval_seconds.is_some() {

                    return Err(
                        "--interval may only be specified once"
                            .to_string()
                    );
                }


                let value =
                    argument_value(
                        args,
                        index,
                        "--interval",
                    )?;


                let seconds =
                    value.parse::<u64>()
                        .map_err(
                            |_| {
                                format!(
                                    "Invalid --interval value '{}'; specify a positive number of seconds",
                                    value
                                )
                            }
                        )?;


                if seconds
                    == 0
                {
                    return Err(
                        "--interval must be greater than zero"
                            .to_string()
                    );
                }


                interval_seconds =
                    Some(
                        seconds
                    );


                index += 2;
            }


            "--family" => {

                return Err(
                    "--family is not valid with --preview-shader; use --texture"
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

            interval_seconds,

            fps,

            animation_speed,
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
             --start\n\
                 Start Screenshaver normally. Equivalent to launching without an option.\n\
         \n\
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
             --preview-texture --family FAMILY[:COUNT] [--palette PALETTE]\n\
                 Preview one procedurally generated texture.\n\
                 COUNT may range from 1 through 1024.\n\
                 This command does not consult screenshaver.toml.\n\
         \n\
             --preview-shader PATH [--interval SECONDS] [--fps FPS] [--speed MULTIPLIER]\n\
                              [--texture FAMILY[:COUNT]] [--palette PALETTE]\n\
                 Preview one shader or all shaders directly inside a folder.\n\
                 Folder previews use a 30-second interval unless overridden.\n\
                 --speed accepts an animation multiplier from 0.01 through 10.0.\n\
                 The default animation speed is 1.0.\n\
                 Command-line texture values override TOML selection values.\n\
                 --texture and --palette are accepted as shorter aliases.\n\
         \n\
             --list-textures\n\
                 Display available procedural texture families.\n\
         \n\
             --list-palettes\n\
                 Display available procedural texture palettes.\n\
         \n\
             --add-override TARGET SHADER PROPERTY [PROPERTY ...]\n\
                 Add a complete screensaver or wallpaper shader override.\n\
                 Properties: texture, palette, fps, speed.\n\
                 PROPERTY may use NAME:VALUE or NAME=VALUE syntax.\n\
         \n             --delete-override TARGET SHADER\n\
                 Delete an existing screensaver or wallpaper shader override.\n\
         \n             --list-overrides [TARGET]\n\
                 List both override tables, or only the selected target.\n\
         \n             --delete-cache\n\
                 Delete all Screenshaver cached/preprocessed shaders.\n\
         \n\
         Texture families:\n\
             bricks\n\
             cells\n\
             clouds\n\
             hexagons\n\
             marble\n\
             mesh\n\
             noise\n\
             radial\n\
         \n\
         Texture palettes:\n\
             brick\n\
             bronze\n\
             lichen\n\
             mist\n\
             sandstone\n\
             slate\n\
         \n\
         Examples:\n\
             screenshaver --start\n\
             screenshaver --stop\n\
             screenshaver --preview-texture --family noise:1024\n\
             screenshaver --preview-texture --family marble --palette sandstone\n\
             screenshaver --preview-shader \"Heartfelt.glsl\"\n\
             screenshaver --preview-shader \"Heartfelt.glsl\" --texture clouds\n\
             screenshaver --preview-shader \"Heartfelt.glsl\" --palette mist\n\
             screenshaver --add-override screensaver CandyWarp.fs texture:bricks palette:mist fps:24 speed:0.5\n\
             screenshaver --delete-override wallpaper CandyWarp.fs\n\
             screenshaver --list-overrides screensaver\n\
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

