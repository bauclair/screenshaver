//! Procedural cloud texture generation.
//!
//! Clouds are generated with layered value noise, also known as
//! fractal Brownian motion. The generator produces normalized
//! values and delegates color mapping to palettes.rs.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Cloud-generation parameters
// ============================================================

const OCTAVES: u32 =
    6;

const BASE_FREQUENCY: f32 =
    3.0;

const FREQUENCY_MULTIPLIER: f32 =
    2.0;

const AMPLITUDE_MULTIPLIER: f32 =
    0.52;

const CONTRAST: f32 =
    1.15;

const BRIGHTNESS: f32 =
    0.02;


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
                    "Cloud texture buffer size overflow"
                        .to_string()
                }
            )?;


    let mut pixels =
        Vec::with_capacity(
            byte_count
        );


    let inverse_size =
        1.0
            / TEXTURE_SIZE as f32;


    for y in
        0..TEXTURE_SIZE
    {
        let normalized_y =
            y as f32
                * inverse_size;


        for x in
            0..TEXTURE_SIZE
        {
            let normalized_x =
                x as f32
                    * inverse_size;


            let value =
                cloud_value(
                    normalized_x,
                    normalized_y,
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
        TextureFamily::Clouds,
        palette,
        seed,
    )
}


// ============================================================
// Cloud field
// ============================================================

fn cloud_value(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let mut frequency =
        BASE_FREQUENCY;


    let mut amplitude =
        1.0_f32;


    let mut total =
        0.0_f32;


    let mut amplitude_total =
        0.0_f32;


    for octave in
        0..OCTAVES
    {
        let octave_seed =
            seed.wrapping_add(
                octave as u64
                    * 0x9E37_79B9
            );


        total +=
            value_noise(
                x * frequency,
                y * frequency,
                octave_seed,
            )
            * amplitude;


        amplitude_total +=
            amplitude;


        frequency *=
            FREQUENCY_MULTIPLIER;


        amplitude *=
            AMPLITUDE_MULTIPLIER;
    }


    let normalized =
        if amplitude_total
            > f32::EPSILON
        {
            total
                / amplitude_total

        } else {

            0.0
        };


    apply_tone_curve(
        normalized
    )
}


fn apply_tone_curve(
    value: f32,
) -> f32 {

    let centered =
        (
            value - 0.5
        )
        * CONTRAST
        + 0.5
        + BRIGHTNESS;


    smoothstep(
        0.0,
        1.0,
        centered.clamp(
            0.0,
            1.0,
        ),
    )
}


// ============================================================
// Value noise
// ============================================================

fn value_noise(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let x0 =
        x.floor() as i32;


    let y0 =
        y.floor() as i32;


    let x1 =
        x0 + 1;


    let y1 =
        y0 + 1;


    let fractional_x =
        x - x.floor();


    let fractional_y =
        y - y.floor();


    let smooth_x =
        fade(
            fractional_x
        );


    let smooth_y =
        fade(
            fractional_y
        );


    let value_00 =
        lattice_value(
            x0,
            y0,
            seed,
        );


    let value_10 =
        lattice_value(
            x1,
            y0,
            seed,
        );


    let value_01 =
        lattice_value(
            x0,
            y1,
            seed,
        );


    let value_11 =
        lattice_value(
            x1,
            y1,
            seed,
        );


    let lower =
        interpolate(
            value_00,
            value_10,
            smooth_x,
        );


    let upper =
        interpolate(
            value_01,
            value_11,
            smooth_x,
        );


    interpolate(
        lower,
        upper,
        smooth_y,
    )
}


fn lattice_value(
    x: i32,
    y: i32,
    seed: u64,
) -> f32 {

    let x_bits =
        x as i64
            as u64;


    let y_bits =
        y as i64
            as u64;


    let mut value =
        seed
            ^ x_bits.wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ y_bits.wrapping_mul(
                0xC2B2_AE3D_27D4_EB4F
            );


    value =
        mix_u64(
            value
        );


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
// Mathematical helpers
// ============================================================

fn fade(
    value: f32,
) -> f32 {

    value
        * value
        * value
        * (
            value
                * (
                    value * 6.0
                        - 15.0
                )
                + 10.0
        )
}


fn interpolate(
    start: f32,
    end: f32,
    amount: f32,
) -> f32 {

    start
        + (
            end - start
        )
        * amount
}


fn smoothstep(
    edge_start: f32,
    edge_end: f32,
    value: f32,
) -> f32 {

    if (
        edge_end - edge_start
    )
    .abs()
        <= f32::EPSILON
    {
        return 0.0;
    }


    let normalized =
        (
            (
                value - edge_start
            )
            / (
                edge_end - edge_start
            )
        )
        .clamp(
            0.0,
            1.0,
        );


    normalized
        * normalized
        * (
            3.0
                - 2.0
                    * normalized
        )
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn cloud_value_is_normalized() {

        for y in
            0..16
        {
            for x in
                0..16
            {
                let value =
                    cloud_value(
                        x as f32
                            / 16.0,
                        y as f32
                            / 16.0,
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
            cloud_value(
                0.25,
                0.75,
                999,
            );


        let second =
            cloud_value(
                0.25,
                0.75,
                999,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_output() {

        let first =
            cloud_value(
                0.25,
                0.75,
                1,
            );


        let second =
            cloud_value(
                0.25,
                0.75,
                2,
            );


        assert_ne!(
            first,
            second
        );
    }
}