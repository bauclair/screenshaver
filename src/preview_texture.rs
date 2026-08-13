use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use crate::palettes::PaletteColor;
use crate::parse_texture_specification::TextureSpecification;


pub fn run(
    texture: TextureSpecification,
    palette: Option<String>,
) {

    let logfile =
        crate::locate_paths::runtime_log_path();


    crate::logger::information(
        &logfile,
        &format!(
            "[TEXTURE_PREVIEW] Texture preview requested: {}",
            texture.display_name(),
        ),
    );


    let config =
        match crate::load_config::load_config(
            &crate::locate_paths::config_path()
        ) {

            Ok(result) => result.config,

            Err(error) => {

                crate::logger::error(
                    &logfile,
                    &format!(
                        "[TEXTURE_PREVIEW] Configuration error: {}",
                        error,
                    ),
                );


                eprintln!(
                    "[TEXTURE PREVIEW] Configuration error: {}",
                    error
                );

                return;
            }
        };


    let seed =
        generate_seed();


    let (
        palette,
        palette_source,
    ) =
        match palette {

            Some(name) => {

                match PaletteColor::parse_hex(
                    &name
                ) {

                    Ok(palette) => (
                        palette,
                        "command line",
                    ),

                    Err(error) => {

                        crate::logger::error(
                            &logfile,
                            &format!(
                                "[TEXTURE_PREVIEW] PaletteColor selection failed: {}",
                                error,
                            ),
                        );


                        eprintln!(
                            "[TEXTURE PREVIEW] {}",
                            error
                        );

                        return;
                    }
                }
            }

            None => {

                (
                    random_palette(
                        seed
                    ),
                    "random fallback",
                )
            }
        };


    crate::logger::debug(
        &logfile,
        &format!(
            "[TEXTURE_PREVIEW] PaletteColor selection source: {}",
            palette_source,
        ),
    );


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

                crate::logger::error(
                    &logfile,
                    &format!(
                        "[TEXTURE_PREVIEW] Texture generation failed: {}",
                        error,
                    ),
                );


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
        crate::logger::error(
            &logfile,
            &format!(
                "[TEXTURE_PREVIEW] Texture validation failed: {}",
                error,
            ),
        );


        eprintln!(
            "[TEXTURE PREVIEW] Validation failed: {}",
            error
        );

        return;
    }


    crate::logger::information(
        &logfile,
        &format!(
            "[TEXTURE_PREVIEW] Generated texture: family={}, primitives={}, palette={}, seed={}, size={}x{}, pixels={}, bytes={}",
            texture.specification.family,
            texture.specification.requested_primitive_count,
            texture.palette,
            texture.seed,
            texture.width,
            texture.height,
            texture.pixel_count(),
            texture.byte_count(),
        ),
    );


    println!(
        "[TEXTURE PREVIEW] Generated successfully"
    );


    println!(
        "Family: {}",
        texture.specification.family
    );


    println!(
        "PaletteColor: {}",
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


    crate::logger::information(
        &logfile,
        "[TEXTURE_PREVIEW] Opening texture preview window",
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

            crate::logger::information(
                &logfile,
                "[TEXTURE_PREVIEW] Texture preview window closed",
            );


            println!(
                "[TEXTURE PREVIEW] Preview closed"
            );
        }


        Err(error) => {

            crate::logger::error(
                &logfile,
                &format!(
                    "[TEXTURE_PREVIEW] Texture display failed: {}",
                    error,
                ),
            );


            eprintln!(
                "[TEXTURE PREVIEW] Display failed: {}",
                error
            );
        }
    }

}


fn random_palette(
    seed: u64,
) -> PaletteColor {

    PaletteColor::random_from_seed(
        seed
    )
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

