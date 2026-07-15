//! Procedural noise texture generation.
//!
//! This first Noise profile recreates fine monochrome television
//! snow. Every output pixel receives a deterministic pseudo-random
//! intensity, with restrained horizontal correlation and scanline
//! modulation to keep the result visually similar to analog static.
//!
//! The engine remains deterministic:
//!
//!     same family + palette + seed = same texture

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Television-snow parameters
// ============================================================

/// Number of source-noise samples across the texture.
///
/// Keeping this equal to TEXTURE_SIZE produces approximately
/// one independent static sample per output pixel.
const NOISE_FREQUENCY: u32 =
    TEXTURE_SIZE;

/// Contrast of the underlying white-noise signal.
const NOISE_CONTRAST: f32 =
    1.32;

/// Bias applied after contrast adjustment.
const NOISE_BRIGHTNESS: f32 =
    0.0;

/// Mix of a horizontally adjacent sample.
///
/// A small contribution creates subtle analog-style horizontal
/// correlation without visibly blurring the static.
const HORIZONTAL_CORRELATION: f32 =
    0.10;

/// Mix of the previous scanline's sample.
const VERTICAL_CORRELATION: f32 =
    0.035;

/// Strength of alternating scanline modulation.
const SCANLINE_STRENGTH: f32 =
    0.045;

/// Strength of occasional bright and dark impulses.
const IMPULSE_STRENGTH: f32 =
    0.18;

/// Probability that a pixel receives an impulse.
const IMPULSE_DENSITY: f32 =
    0.018;

/// Gamma-like shaping of the final scalar signal.
///
/// Values above 1.0 emphasize dark and midtone static.
const SIGNAL_SHARPNESS: f32 =
    1.08;


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
) -> Result<GeneratedTexture, String> {

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
                    "Noise texture buffer size overflow"
                        .to_string()
                }
            )?;


    let mut pixels =
        Vec::with_capacity(
            byte_count
        );


    for y in
        0..TEXTURE_SIZE
    {
        for x in
            0..TEXTURE_SIZE
        {
            let value =
                television_snow_value(
                    x,
                    y,
                    seed,
                );


            let color =
                palette.map_rgba(
                    value
                );


            pixels.extend_from_slice(
                &color
            );
        }
    }


    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Noise,
        palette,
        seed,
    )
}


// ============================================================
// Television-snow field
// ============================================================

fn television_snow_value(
    x: u32,
    y: u32,
    seed: u64,
) -> f32 {

    let source_x =
        scaled_coordinate(
            x,
            TEXTURE_SIZE,
            NOISE_FREQUENCY,
        );


    let source_y =
        scaled_coordinate(
            y,
            TEXTURE_SIZE,
            NOISE_FREQUENCY,
        );


    let primary =
        lattice_noise(
            source_x,
            source_y,
            seed,
        );


    let horizontal =
        lattice_noise(
            source_x.wrapping_sub(
                1
            ),
            source_y,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        );


    let vertical =
        lattice_noise(
            source_x,
            source_y.wrapping_sub(
                1
            ),
            seed.wrapping_add(
                0x9E37_79B9_7F4A_7C15
            ),
        );


    let correlated =
        primary
            * (
                1.0
                    - HORIZONTAL_CORRELATION
                    - VERTICAL_CORRELATION
            )
        + horizontal
            * HORIZONTAL_CORRELATION
        + vertical
            * VERTICAL_CORRELATION;


    //---------------------------------------------------------
    // Alternate scanlines receive a small deterministic
    // brightness shift, evoking analog raster structure.
    //---------------------------------------------------------

    let scanline =
        if y
            & 1
            == 0
        {
            SCANLINE_STRENGTH
        } else {
            -SCANLINE_STRENGTH
        };


    //---------------------------------------------------------
    // Sparse impulses reproduce occasional very bright or dark
    // flecks commonly visible in weak analog reception.
    //---------------------------------------------------------

    let impulse_selector =
        lattice_noise(
            source_x,
            source_y,
            seed.wrapping_add(
                0xD1B5_4A32_D192_ED03
            ),
        );


    let impulse_polarity =
        lattice_noise(
            source_x,
            source_y,
            seed.wrapping_add(
                0x94D0_49BB_1331_11EB
            ),
        );


    let impulse =
        if impulse_selector
            < IMPULSE_DENSITY
        {
            if impulse_polarity
                < 0.5
            {
                -IMPULSE_STRENGTH
            } else {
                IMPULSE_STRENGTH
            }

        } else {

            0.0
        };


    let contrasted =
        (
            correlated
                - 0.5
        )
        * NOISE_CONTRAST
        + 0.5
        + NOISE_BRIGHTNESS
        + scanline
        + impulse;


    contrasted
        .clamp(
            0.0,
            1.0,
        )
        .powf(
            SIGNAL_SHARPNESS
        )
}


// ============================================================
// Coordinate scaling
// ============================================================

fn scaled_coordinate(
    coordinate: u32,
    output_size: u32,
    source_frequency: u32,
) -> u32 {

    if output_size
        == 0
    {
        return 0;
    }


    (
        coordinate as u64
            * source_frequency as u64
        / output_size as u64
    ) as u32
}


// ============================================================
// Deterministic lattice noise
// ============================================================

fn lattice_noise(
    x: u32,
    y: u32,
    seed: u64,
) -> f32 {

    let value =
        seed
            ^ (
                x as u64
            )
            .wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ (
                y as u64
            )
            .wrapping_mul(
                0xC2B2_AE3D_27D4_EB4F
            );


    hash_to_unit_float(
        mix_u64(
            value
        )
    )
}


// ============================================================
// Hashing
// ============================================================

fn hash_to_unit_float(
    value: u64,
) -> f32 {

    let upper =
        (
            value >> 40
        ) as u32;


    upper as f32
        / 16_777_215.0
}


fn mix_u64(
    mut value: u64,
) -> u64 {

    value ^=
        value >> 30;


    value =
        value.wrapping_mul(
            0xBF58_476D_1CE4_E5B9
        );


    value ^=
        value >> 27;


    value =
        value.wrapping_mul(
            0x94D0_49BB_1331_11EB
        );


    value ^=
        value >> 31;


    value
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn television_snow_value_is_normalized() {

        for y in
            0..64
        {
            for x in
                0..64
            {
                let value =
                    television_snow_value(
                        x,
                        y,
                        12345,
                    );


                assert!(
                    (
                        0.0..=1.0
                    )
                    .contains(
                        &value
                    )
                );
            }
        }
    }


    #[test]
    fn same_seed_is_deterministic() {

        let first =
            television_snow_value(
                371,
                629,
                999,
            );


        let second =
            television_snow_value(
                371,
                629,
                999,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_output() {

        let mut found_difference =
            false;


        for y in
            0..32
        {
            for x in
                0..32
            {
                let first =
                    television_snow_value(
                        x,
                        y,
                        1,
                    );


                let second =
                    television_snow_value(
                        x,
                        y,
                        2,
                    );


                if (
                    first - second
                )
                .abs()
                    > f32::EPSILON
                {
                    found_difference =
                        true;

                    break;
                }
            }


            if found_difference
            {
                break;
            }
        }


        assert!(
            found_difference,
            "Different seeds produced identical television snow"
        );
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                Palette::Slate,
                12345,
            )
            .expect(
                "noise generation"
            );


        assert_eq!(
            texture.family,
            TextureFamily::Noise
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

