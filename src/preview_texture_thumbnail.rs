//! Static Control Center texture-thumbnail generation.
//!
//! This module deliberately does not participate in runtime texture animation.
//! In particular, the Eyes texture is generated in its ordinary all-open
//! state and remains static in the Textures-tab thumbnail.
//!
//! Full-resolution production texture generation remains 1024x1024.  The
//! resulting RGBA image is then downsampled with Lanczos3 for display in egui.

use image::imageops::{
    self,
    FilterType,
};

use image::RgbaImage;

use crate::generate_textures::GeneratedTexture;
use crate::palettes::PaletteColor;
use crate::parse_texture_specification::TextureSpecification;


// ============================================================
// Thumbnail result
// ============================================================

pub(crate) struct TextureThumbnail {

    pub color_image:
        egui::ColorImage,

    pub source_width:
        u32,

    pub source_height:
        u32,

    pub preview_width:
        u32,

    pub preview_height:
        u32,

    pub seed:
        u64,
}


// ============================================================
// Public generation API
// ============================================================

pub(crate) fn generate(
    specification: &TextureSpecification,
    palette: PaletteColor,
    seed: u64,
    maximum_preview_size: u32,
) -> Result<TextureThumbnail, String> {

    let generated =
        crate::generate_textures::generate_from_specification(
            specification,
            palette,
            seed,
        )?;


    generated.validate_standard()?;


    from_generated_texture(
        &generated,
        maximum_preview_size,
    )
}


// ============================================================
// Conversion / downsampling
// ============================================================

fn from_generated_texture(
    generated: &GeneratedTexture,
    maximum_preview_size: u32,
) -> Result<TextureThumbnail, String> {

    if maximum_preview_size == 0 {
        return Err(
            "Texture thumbnail size must be greater than zero"
                .to_string()
        );
    }


    let source =
        RgbaImage::from_raw(
            generated.width,
            generated.height,
            generated.pixels.clone(),
        )
        .ok_or_else(
            || {
                format!(
                    "Unable to construct {}x{} RGBA image for texture thumbnail",
                    generated.width,
                    generated.height,
                )
            }
        )?;


    let (
        preview_width,
        preview_height,
    ) =
        fit_dimensions(
            generated.width,
            generated.height,
            maximum_preview_size,
        );


    let resized =
        if preview_width
            == generated.width
            && preview_height
                == generated.height
        {
            source

        } else {
            imageops::resize(
                &source,
                preview_width,
                preview_height,
                FilterType::Lanczos3,
            )
        };


    let color_image =
        egui::ColorImage::from_rgba_unmultiplied(
            [
                preview_width as usize,
                preview_height as usize,
            ],
            resized.as_raw(),
        );


    Ok(
        TextureThumbnail {
            color_image,

            source_width:
                generated.width,

            source_height:
                generated.height,

            preview_width,

            preview_height,

            seed:
                generated.seed,
        }
    )
}


fn fit_dimensions(
    width: u32,
    height: u32,
    maximum_size: u32,
) -> (
    u32,
    u32,
) {

    if width == 0
        || height == 0
    {
        return (
            1,
            1,
        );
    }


    if width <= maximum_size
        && height <= maximum_size
    {
        return (
            width,
            height,
        );
    }


    let width_scale =
        maximum_size as f64
            / width as f64;


    let height_scale =
        maximum_size as f64
            / height as f64;


    let scale =
        width_scale.min(
            height_scale
        );


    let resized_width =
        (
            width as f64
                * scale
        )
        .round()
        .clamp(
            1.0,
            maximum_size as f64,
        )
        as u32;


    let resized_height =
        (
            height as f64
                * scale
        )
        .round()
        .clamp(
            1.0,
            maximum_size as f64,
        )
        as u32;


    (
        resized_width,
        resized_height,
    )
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn square_source_fits_square_preview() {

        assert_eq!(
            fit_dimensions(
                1024,
                1024,
                240,
            ),
            (
                240,
                240,
            )
        );
    }


    #[test]
    fn rectangular_source_preserves_aspect_ratio() {

        assert_eq!(
            fit_dimensions(
                1024,
                512,
                240,
            ),
            (
                240,
                120,
            )
        );
    }


    #[test]
    fn smaller_source_is_not_upscaled() {

        assert_eq!(
            fit_dimensions(
                128,
                128,
                240,
            ),
            (
                128,
                128,
            )
        );
    }
}
