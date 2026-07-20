use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use crate::palettes::Palette;
use crate::parse_texture_specification::TextureSpecification;


pub fn run(
    texture: TextureSpecification,
    palette: Option<String>,
) {

    let config =
        match crate::load_config::load_config(
            &crate::locate_paths::config_path()
        ) {

            Ok(result) => result.config,

            Err(error) => {

                eprintln!(
                    "[TEXTURE PREVIEW] Configuration error: {}",
                    error
                );

                return;
            }
        };


    let seed =
        generate_seed();


    let palette =
        match palette {

            Some(name) => {

                match Palette::from_name(
                    &name
                ) {

                    Ok(palette) => palette,

                    Err(error) => {

                        eprintln!(
                            "[TEXTURE PREVIEW] {}",
                            error
                        );

                        return;
                    }
                }
            }

            None => {

                random_palette(
                    seed
                )
            }
        };


    println!(
        "[TEXTURE PREVIEW] Generating texture..."
    );


    let texture =
        match crate::generate_textures::generate_from_specification(
            &texture,
            palette,
            seed,
        ) {

            Ok(texture) => texture,

            Err(error) => {

                eprintln!(
                    "[TEXTURE PREVIEW] Generation failed: {}",
                    error
                );

                return;
            }
        };


    if let Err(error) =
        texture.validate_standard()
    {
        eprintln!(
            "[TEXTURE PREVIEW] Validation failed: {}",
            error
        );

        return;
    }


    println!(
        "[TEXTURE PREVIEW] Generated successfully"
    );


    println!(
        "Family: {}",
        texture.family
    );


    println!(
        "Palette: {}",
        texture.palette
    );


    println!(
        "Seed: {}",
        texture.seed
    );


    println!(
        "Size: {}x{}",
        texture.width,
        texture.height
    );


    println!(
        "Pixels: {}",
        texture.pixel_count()
    );


    println!(
        "Bytes: {}",
        texture.byte_count()
    );

    println!(
        "[TEXTURE PREVIEW] Opening preview window..."
    );


    match crate::display_texture::display(
        &texture,
        config.subtitles,
        config.subtitle_placement,
    ) {

        Ok(()) => {

            println!(
                "[TEXTURE PREVIEW] Preview closed"
            );
        }


        Err(error) => {

            eprintln!(
                "[TEXTURE PREVIEW] Display failed: {}",
                error
            );
        }
    }

}


fn random_palette(
    seed: u64,
) -> Palette {

    let palettes = [
        Palette::Slate,
        Palette::Sandstone,
        Palette::Lichen,
        Palette::Mist,
        Palette::Bronze,
        Palette::Brick,
    ];


    let index =
        (
            seed
                % palettes.len()
                    as u64
        ) as usize;


    palettes[
        index
    ]
}


fn generate_seed() -> u64 {

    SystemTime::now()
        .duration_since(
            UNIX_EPOCH
        )
        .map(
            |duration| {
                duration.as_nanos()
                    as u64
            }
        )
        .unwrap_or(
            0
        )
}

