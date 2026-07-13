use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use crate::generate_textures::TextureFamily;
use crate::palettes::Palette;


pub fn run(
    family: String,
    palette: Option<String>,
) {

    let family =
        match TextureFamily::from_name(
            &family
        ) {

            Ok(family) => family,

            Err(error) => {

                eprintln!(
                    "[TEXTURE PREVIEW] {}",
                    error
                );

                return;
            }
        };


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

                //-------------------------------------------------
                // Temporary default.
                //
                // Random palette selection will eventually belong
                // in manage_textures.rs.
                //-------------------------------------------------

                Palette::Mist
            }
        };


    let seed =
        generate_seed();


    println!(
        "[TEXTURE PREVIEW] Generating texture..."
    );


    let texture =
        match crate::generate_textures::generate(
            family,
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
        &texture
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