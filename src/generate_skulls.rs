//! Photorealistic skull texture generation.
//!
//! The Skulls engine stamps one embedded photographic skull image into a
//! deterministic, tightly packed running-bond lattice:
//!
//! - the source PNG is embedded into the executable at compile time;
//! - transparent margins are cropped before scaling;
//! - the requested primitive count controls approximate visual density;
//! - rows are tightly packed and alternating rows are shifted by half a skull;
//! - the lattice extends beyond all texture edges so boundary skulls clip
//!   naturally, just as shifted bricks do in the Bricks texture;
//! - the source photograph is converted to luminance and mapped through the
//!   active Screenshaver palette while preserving photographic shading;
//! - the skull image is resized only once for each generated texture;
//! - placement is deterministic; `seed` remains part of the standard texture
//!   generator contract but does not currently alter the layout.

use image::imageops::{
    self,
    FilterType,
};

use image::{
    ImageFormat,
    RgbaImage,
};

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::PaletteColor;


// ============================================================
// Embedded source asset
// ============================================================

/// The project should store the supplied master image at:
///
///     assets/textures/skull.png
///
/// `generate_skulls.rs` lives in `src/`, so the path below is relative to
/// this source file at compile time.
const SKULL_PNG:
    &[u8] =
    include_bytes!(
        "../assets/textures/skull.png"
    );


// ============================================================
// Skull-layout parameters
// ============================================================

/// Skulls are intentionally packed very closely. Values below 1.0 allow the
/// transparent/irregular edges of adjacent skulls to interlock slightly.
const HORIZONTAL_PITCH_FACTOR:
    f32 =
    0.96;

/// Vertical packing is slightly tighter than horizontal packing. This gives
/// the wall-like density of the approved staggered sample without introducing
/// broad horizontal gaps between rows.
const VERTICAL_PITCH_FACTOR:
    f32 =
    0.92;

/// Ignore essentially invisible alpha noise when finding the useful bounds of
/// the master PNG.
const SOURCE_ALPHA_CROP_THRESHOLD:
    u8 =
    2;

/// Extra rows/columns generated outside the visible image. Their skulls are
/// naturally clipped by the 1024x1024 output buffer.
const LATTICE_MARGIN:
    i32 =
    3;

/// Skulls are a deliberately dense texture family. The UI already constrains
/// selectable primitive counts to powers of two. Keep the generator defensive
/// if called directly.
pub const MIN_SKULL_COUNT:
    usize =
    2;

pub const MAX_SKULL_COUNT:
    usize =
    crate::define_constants::MAX_TEXTURE_PRIMITIVES;


// ============================================================
// Internal layout
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct SkullLayout {
    skull_width:
        u32,

    skull_height:
        u32,

    horizontal_pitch:
        f32,

    vertical_pitch:
        f32,
}


impl SkullLayout {

    fn new(
        source_width: u32,
        source_height: u32,
        requested_primitive_count: usize,
    ) -> Result<Self, String> {

        if source_width == 0
            || source_height == 0
        {
            return Err(
                "Skull source image has zero width or height"
                    .to_string()
            );
        }


        let requested =
            requested_primitive_count
                .clamp(
                    MIN_SKULL_COUNT,
                    MAX_SKULL_COUNT,
                ) as f32;


        let source_aspect =
            source_width as f32
                / source_height as f32;


        //---------------------------------------------------------
        // Approximate density:
        //
        //     requested ~= texture_area / (pitch_x * pitch_y)
        //
        // with:
        //
        //     pitch_x = skull_width  * HORIZONTAL_PITCH_FACTOR
        //     pitch_y = skull_height * VERTICAL_PITCH_FACTOR
        //     skull_width = skull_height * source_aspect
        //
        // Solving this gives a skull size whose infinite staggered lattice
        // occupies approximately the requested visual density.
        //---------------------------------------------------------

        let texture_area =
            TEXTURE_SIZE as f32
                * TEXTURE_SIZE as f32;


        let denominator =
            requested
                * source_aspect
                * HORIZONTAL_PITCH_FACTOR
                * VERTICAL_PITCH_FACTOR;


        let skull_height =
            (
                texture_area
                    / denominator.max(
                        f32::EPSILON
                    )
            )
            .sqrt()
            .round()
            .clamp(
                1.0,
                TEXTURE_SIZE as f32,
            )
            as u32;


        let skull_width =
            (
                skull_height as f32
                    * source_aspect
            )
            .round()
            .clamp(
                1.0,
                TEXTURE_SIZE as f32,
            )
            as u32;


        let horizontal_pitch =
            (
                skull_width as f32
                    * HORIZONTAL_PITCH_FACTOR
            )
            .max(
                1.0
            );


        let vertical_pitch =
            (
                skull_height as f32
                    * VERTICAL_PITCH_FACTOR
            )
            .max(
                1.0
            );


        Ok(
            Self {
                skull_width,
                skull_height,
                horizontal_pitch,
                vertical_pitch,
            }
        )
    }
}


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: PaletteColor,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    //---------------------------------------------------------
    // Preserve the standard generator contract. Skulls are
    // intentionally deterministic in this first implementation.
    //---------------------------------------------------------

    let _ =
        seed;


    let source =
        load_cropped_source()?;


    let layout =
        SkullLayout::new(
            source.width(),
            source.height(),
            requested_primitive_count,
        )?;


    //---------------------------------------------------------
    // Resize the photographic master exactly once for this
    // generated texture. Lanczos3 preserves the strongest skull
    // details at the very small 512/1024-count sizes.
    //---------------------------------------------------------

    let resized =
        imageops::resize(
            &source,
            layout.skull_width,
            layout.skull_height,
            FilterType::Lanczos3,
        );


    let palette_mapped =
        palette_map_skull(
            &resized,
            palette,
        );


    let pixel_count =
        TEXTURE_SIZE as usize
            * TEXTURE_SIZE as usize;


    let byte_count =
        pixel_count
            .checked_mul(
                4
            )
            .ok_or_else(
                || {
                    "Skull texture buffer size overflow"
                        .to_string()
                }
            )?;


    //---------------------------------------------------------
    // The exposed background is opaque black. Transparent edge
    // pixels in the source skull are alpha-composited into it.
    //---------------------------------------------------------

    let mut pixels =
        vec![
            0_u8;
            byte_count
        ];


    for pixel in
        pixels.chunks_exact_mut(
            4
        )
    {
        pixel[3] =
            255;
    }


    stamp_running_bond_lattice(
        &mut pixels,
        &palette_mapped,
        &layout,
    );


    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Skulls,
        palette,
        seed,
    )
}


// ============================================================
// Source image preparation
// ============================================================

fn load_cropped_source()
    -> Result<RgbaImage, String>
{
    let decoded =
        image::load_from_memory_with_format(
            SKULL_PNG,
            ImageFormat::Png,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to decode embedded skull PNG: {}",
                    error
                )
            }
        )?
        .to_rgba8();


    crop_transparent_margins(
        &decoded
    )
}


fn crop_transparent_margins(
    source: &RgbaImage,
) -> Result<RgbaImage, String> {

    let mut minimum_x =
        source.width();

    let mut minimum_y =
        source.height();

    let mut maximum_x =
        0_u32;

    let mut maximum_y =
        0_u32;

    let mut found_visible_pixel =
        false;


    for (
        x,
        y,
        pixel,
    ) in
        source.enumerate_pixels()
    {
        if pixel[3]
            <= SOURCE_ALPHA_CROP_THRESHOLD
        {
            continue;
        }


        found_visible_pixel =
            true;

        minimum_x =
            minimum_x.min(
                x
            );

        minimum_y =
            minimum_y.min(
                y
            );

        maximum_x =
            maximum_x.max(
                x
            );

        maximum_y =
            maximum_y.max(
                y
            );
    }


    if !found_visible_pixel {
        return Err(
            "Embedded skull PNG contains no visible pixels"
                .to_string()
        );
    }


    let crop_width =
        maximum_x
            - minimum_x
            + 1;

    let crop_height =
        maximum_y
            - minimum_y
            + 1;


    Ok(
        imageops::crop_imm(
            source,
            minimum_x,
            minimum_y,
            crop_width,
            crop_height,
        )
        .to_image()
    )
}


// ============================================================
// Palette conversion
// ============================================================

fn palette_map_skull(
    source: &RgbaImage,
    palette: PaletteColor,
) -> RgbaImage {

    let mut mapped =
        RgbaImage::new(
            source.width(),
            source.height(),
        );


    for (
        x,
        y,
        source_pixel,
    ) in
        source.enumerate_pixels()
    {
        let alpha =
            source_pixel[3];


        if alpha == 0 {
            mapped.put_pixel(
                x,
                y,
                image::Rgba(
                    [
                        0,
                        0,
                        0,
                        0,
                    ]
                ),
            );

            continue;
        }


        let luminance =
            source_luminance(
                source_pixel[0],
                source_pixel[1],
                source_pixel[2],
            );


        let mut color =
            palette.map_rgba(
                luminance
            );


        color[3] =
            alpha;


        mapped.put_pixel(
            x,
            y,
            image::Rgba(
                color
            ),
        );
    }


    mapped
}


fn source_luminance(
    red: u8,
    green: u8,
    blue: u8,
) -> f32 {

    //---------------------------------------------------------
    // Rec. 709 / sRGB luminance weights. For texture artwork,
    // this preserves the perceived photographic light/dark
    // structure before palette remapping.
    //---------------------------------------------------------

    (
        red as f32
            * 0.2126
        + green as f32
            * 0.7152
        + blue as f32
            * 0.0722
    )
        / 255.0
}


// ============================================================
// Running-bond image placement
// ============================================================

fn stamp_running_bond_lattice(
    destination: &mut [u8],
    skull: &RgbaImage,
    layout: &SkullLayout,
) {

    let texture_size =
        TEXTURE_SIZE as f32;


    let maximum_rows =
        (
            texture_size
                / layout.vertical_pitch
        )
        .ceil() as i32
        + LATTICE_MARGIN;


    let maximum_columns =
        (
            texture_size
                / layout.horizontal_pitch
        )
        .ceil() as i32
        + LATTICE_MARGIN;


    //---------------------------------------------------------
    // Center the infinite lattice on the texture. Odd rows move
    // by half a horizontal pitch, exactly like a running-bond
    // brick wall. Iterating beyond the visible boundaries gives
    // clipped half-skulls at the edges instead of empty margins.
    //---------------------------------------------------------

    for row in
        -maximum_rows
            ..=
        maximum_rows
    {
        let row_offset =
            if row
                & 1
                == 0
            {
                0.0
            } else {
                layout.horizontal_pitch
                    * 0.5
            };


        let center_y =
            texture_size
                * 0.5
            + row as f32
                * layout.vertical_pitch;


        for column in
            -maximum_columns
                ..=
            maximum_columns
        {
            let center_x =
                texture_size
                    * 0.5
                + column as f32
                    * layout.horizontal_pitch
                + row_offset;


            let left =
                (
                    center_x
                        - skull.width() as f32
                            * 0.5
                )
                .round() as i32;


            let top =
                (
                    center_y
                        - skull.height() as f32
                            * 0.5
                )
                .round() as i32;


            stamp_skull(
                destination,
                skull,
                left,
                top,
            );
        }
    }
}


fn stamp_skull(
    destination: &mut [u8],
    skull: &RgbaImage,
    destination_left: i32,
    destination_top: i32,
) {

    for source_y in
        0..skull.height()
    {
        let destination_y =
            destination_top
                + source_y as i32;


        if destination_y < 0
            || destination_y
                >= TEXTURE_SIZE as i32
        {
            continue;
        }


        for source_x in
            0..skull.width()
        {
            let destination_x =
                destination_left
                    + source_x as i32;


            if destination_x < 0
                || destination_x
                    >= TEXTURE_SIZE as i32
            {
                continue;
            }


            let source_pixel =
                skull.get_pixel(
                    source_x,
                    source_y,
                );


            let source_alpha =
                source_pixel[3] as u32;


            if source_alpha == 0 {
                continue;
            }


            let destination_index =
                (
                    destination_y as usize
                        * TEXTURE_SIZE as usize
                    + destination_x as usize
                )
                    * 4;


            //-------------------------------------------------
            // Standard straight-alpha "source over" blend.
            // Destination alpha is always opaque, so its RGB
            // expression simplifies considerably.
            //-------------------------------------------------

            let inverse_alpha =
                255_u32
                    - source_alpha;


            for channel in
                0..3
            {
                let source_value =
                    source_pixel[
                        channel
                    ] as u32;


                let destination_value =
                    destination[
                        destination_index
                            + channel
                    ] as u32;


                destination[
                    destination_index
                        + channel
                ] =
                    (
                        source_value
                            * source_alpha
                        + destination_value
                            * inverse_alpha
                        + 127
                    )
                        .div_euclid(
                            255
                        )
                        .min(
                            255
                        ) as u8;
            }


            destination[
                destination_index
                    + 3
            ] =
                255;
        }
    }
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn embedded_skull_decodes_and_crops() {

        let skull =
            load_cropped_source()
                .expect(
                    "embedded skull PNG"
                );


        assert!(
            skull.width()
                > 0
        );

        assert!(
            skull.height()
                > 0
        );

        assert!(
            skull.width()
                <= 1261
        );

        assert!(
            skull.height()
                <= 1247
        );
    }


    #[test]
    fn requested_count_is_clamped_to_skull_range() {

        let source =
            load_cropped_source()
                .expect(
                    "embedded skull PNG"
                );


        let minimum =
            SkullLayout::new(
                source.width(),
                source.height(),
                0,
            )
            .expect(
                "minimum skull layout"
            );


        let explicit_minimum =
            SkullLayout::new(
                source.width(),
                source.height(),
                MIN_SKULL_COUNT,
            )
            .expect(
                "explicit minimum skull layout"
            );


        assert_eq!(
            minimum.skull_width,
            explicit_minimum.skull_width
        );

        assert_eq!(
            minimum.skull_height,
            explicit_minimum.skull_height
        );


        let maximum =
            SkullLayout::new(
                source.width(),
                source.height(),
                usize::MAX,
            )
            .expect(
                "maximum skull layout"
            );


        let explicit_maximum =
            SkullLayout::new(
                source.width(),
                source.height(),
                MAX_SKULL_COUNT,
            )
            .expect(
                "explicit maximum skull layout"
            );


        assert_eq!(
            maximum.skull_width,
            explicit_maximum.skull_width
        );

        assert_eq!(
            maximum.skull_height,
            explicit_maximum.skull_height
        );
    }


    #[test]
    fn denser_request_produces_smaller_skulls() {

        let source =
            load_cropped_source()
                .expect(
                    "embedded skull PNG"
                );


        let sparse =
            SkullLayout::new(
                source.width(),
                source.height(),
                2,
            )
            .expect(
                "sparse skull layout"
            );


        let dense =
            SkullLayout::new(
                source.width(),
                source.height(),
                1024,
            )
            .expect(
                "dense skull layout"
            );


        assert!(
            dense.skull_width
                < sparse.skull_width
        );

        assert!(
            dense.skull_height
                < sparse.skull_height
        );
    }


    #[test]
    fn same_request_is_deterministic() {

        let palette =
            PaletteColor::new(
                180,
                180,
                180,
            );


        let first =
            generate(
                palette,
                123,
                64,
            )
            .expect(
                "first skull texture"
            );


        let second =
            generate(
                palette,
                123,
                64,
            )
            .expect(
                "second skull texture"
            );


        assert_eq!(
            first.pixels,
            second.pixels
        );
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                PaletteColor::new(
                    180,
                    180,
                    180,
                ),
                12345,
                64,
            )
            .expect(
                "skull generation"
            );


        assert_eq!(
            texture.specification.family,
            TextureFamily::Skulls
        );


        assert_eq!(
            texture.width,
            TEXTURE_SIZE
        );


        assert_eq!(
            texture.height,
            TEXTURE_SIZE
        );


        assert!(
            texture
                .validate_standard()
                .is_ok()
        );
    }
}
