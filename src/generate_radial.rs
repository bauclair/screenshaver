//! Procedural radial texture generation.
//!
//! The Radial engine generates patterns from polar coordinates.
//! A single generalized phase field can produce concentric rings,
//! straight sunburst rays, spirals, pinwheels, rosettes, and warped
//! radial interference patterns.

use std::f32::consts::TAU;

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Radial-generation parameters
// ============================================================

#[derive(Clone, Copy, Debug)]
struct RadialLayout {
    radial_frequency:f32,
    angular_frequency:f32,
}
impl RadialLayout{
    fn new(requested_primitive_count:usize)->Self{
        let f=(requested_primitive_count.max(1) as f32).sqrt().max(1.0);
        Self{
            radial_frequency:f,
            angular_frequency:
                (
                    f * 0.64
                )
                .round()
                .max(
                    1.0
                ),
        }
    }
}

/// Additional nonlinear curvature applied as radius increases.
/// Positive and negative values reverse the apparent twist.
const CURVATURE: f32 =
    1.85;

/// Rotation of the entire pattern in degrees.
const ROTATION_DEGREES: f32 =
    0.0;

/// Location of the radial center.
const CENTER_X: f32 =
    0.5;

const CENTER_Y: f32 =
    0.5;

/// Axis scaling applied before polar conversion.
const SCALE_X: f32 =
    1.0;

const SCALE_Y: f32 =
    1.0;

/// Width of the bright bands within each phase cycle.
const BAND_WIDTH: f32 =
    0.46;

/// Softness of band transitions.
const EDGE_SOFTNESS: f32 =
    0.055;

/// Shapes brightness inside each band.
const BAND_SHARPNESS: f32 =
    1.30;

/// Strength of low-frequency radial and angular distortion.
const RADIAL_WARP_STRENGTH: f32 =
    0.065;

const ANGULAR_WARP_STRENGTH: f32 =
    0.13;

/// Frequency and octave behavior of the distortion field.
const WARP_FREQUENCY: f32 =
    2.15;

const WARP_OCTAVES: u32 =
    4;

const WARP_FREQUENCY_MULTIPLIER: f32 =
    2.0;

const WARP_AMPLITUDE_MULTIPLIER: f32 =
    0.52;

/// Subtle variation inside visible bands.
const SURFACE_VARIATION_STRENGTH: f32 =
    0.045;

const SURFACE_VARIATION_FREQUENCY: f32 =
    9.0;

/// Final tone adjustment.
const CONTRAST: f32 =
    1.08;

const BRIGHTNESS: f32 =
    0.0;


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    let layout = RadialLayout::new(requested_primitive_count);

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
                    "Radial texture buffer size overflow"
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
            (
                y as f32
                    + 0.5
            )
            * inverse_size;


        for x in
            0..TEXTURE_SIZE
        {
            let normalized_x =
                (
                    x as f32
                        + 0.5
                )
                * inverse_size;


            let value =
                radial_value(
                    normalized_x,
                    normalized_y,
                    seed,
                    &layout,
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
        TextureFamily::Radial,
        palette,
        seed,
    )
}


// ============================================================
// Radial field
// ============================================================

fn radial_value(
    x: f32,
    y: f32,
    seed: u64,
    layout:&RadialLayout,
) -> f32 {

    let rotation =
        ROTATION_DEGREES
            .to_radians();


    let cosine =
        rotation.cos();


    let sine =
        rotation.sin();


    let centered_x =
        (
            x - CENTER_X
        )
        * SCALE_X;


    let centered_y =
        (
            y - CENTER_Y
        )
        * SCALE_Y;


    let rotated_x =
        centered_x
            * cosine
        - centered_y
            * sine;


    let rotated_y =
        centered_x
            * sine
        + centered_y
            * cosine;


    let base_radius =
        (
            rotated_x
                * rotated_x
            + rotated_y
                * rotated_y
        )
        .sqrt();


    let base_angle =
        rotated_y
            .atan2(
                rotated_x
            );


    let radial_warp =
        fractal_noise(
            x * WARP_FREQUENCY + 17.31,
            y * WARP_FREQUENCY - 9.47,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        )
        - 0.5;


    let angular_warp =
        fractal_noise(
            x * WARP_FREQUENCY - 23.18,
            y * WARP_FREQUENCY + 14.62,
            seed.wrapping_add(
                0x9E37_79B9_7F4A_7C15
            ),
        )
        - 0.5;


    let radius =
        (
            base_radius
                + radial_warp
                    * RADIAL_WARP_STRENGTH
        )
        .max(
            0.0
        );


    let angle =
        base_angle
            + angular_warp
                * ANGULAR_WARP_STRENGTH;


    let phase =
        radius
            * layout.radial_frequency
            * TAU
        + angle
            * layout.angular_frequency
        + radius
            * radius
            * CURVATURE
            * TAU;


    let cycle =
        phase
            .rem_euclid(
                TAU
            )
            / TAU;


    let band =
        periodic_band(
            cycle,
            BAND_WIDTH,
            EDGE_SOFTNESS,
        )
        .powf(
            BAND_SHARPNESS
        );


    let surface =
        fractal_noise(
            x * SURFACE_VARIATION_FREQUENCY + 41.0,
            y * SURFACE_VARIATION_FREQUENCY - 27.0,
            seed.wrapping_add(
                0xD1B5_4A32_D192_ED03
            ),
        );


    let varied =
        band
            + (
                surface - 0.5
            )
                * SURFACE_VARIATION_STRENGTH;


    apply_tone_curve(
        varied.clamp(
            0.0,
            1.0,
        )
    )
}


// ============================================================
// Band shaping
// ============================================================

fn periodic_band(
    cycle: f32,
    width: f32,
    softness: f32,
) -> f32 {

    let centered_distance =
        (
            cycle - 0.5
        )
        .abs()
        * 2.0;


    let half_width =
        width
            .clamp(
                0.001,
                0.999,
            );


    let transition_start =
        (
            half_width
                - softness
        )
        .max(
            0.0
        );


    let transition_end =
        (
            half_width
                + softness
        )
        .min(
            1.0
        );


    1.0
        - smoothstep(
            transition_start,
            transition_end,
            centered_distance,
        )
}


// ============================================================
// Layered value noise
// ============================================================

fn fractal_noise(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let mut frequency =
        1.0_f32;


    let mut amplitude =
        1.0_f32;


    let mut total =
        0.0_f32;


    let mut amplitude_total =
        0.0_f32;


    for octave in
        0..WARP_OCTAVES
    {
        let octave_seed =
            seed.wrapping_add(
                (
                    octave as u64
                )
                .wrapping_mul(
                    0x9E37_79B9
                )
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
            WARP_FREQUENCY_MULTIPLIER;


        amplitude *=
            WARP_AMPLITUDE_MULTIPLIER;
    }


    if amplitude_total
        <= f32::EPSILON
    {
        0.0

    } else {

        total
            / amplitude_total
    }
}


fn value_noise(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let floor_x =
        x.floor();


    let floor_y =
        y.floor();


    let x0 =
        floor_x as i32;


    let y0 =
        floor_y as i32;


    let x1 =
        x0 + 1;


    let y1 =
        y0 + 1;


    let fractional_x =
        x - floor_x;


    let fractional_y =
        y - floor_y;


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

    let value =
        seed
            ^ (
                x as i64
                    as u64
            )
            .wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ (
                y as i64
                    as u64
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
// Hashing and interpolation
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


// ============================================================
// Tone mapping
// ============================================================

fn apply_tone_curve(
    value: f32,
) -> f32 {

    let centered =
        (
            value
                - 0.5
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


fn smoothstep(
    edge_start: f32,
    edge_end: f32,
    value: f32,
) -> f32 {

    if (
        edge_end
            - edge_start
    )
    .abs()
        <= f32::EPSILON
    {
        return 0.0;
    }


    let normalized =
        (
            (
                value
                    - edge_start
            )
            / (
                edge_end
                    - edge_start
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
    fn radial_value_is_normalized() {

        let layout=RadialLayout::new(144);


        for y in
            0..32
        {
            for x in
                0..32
            {
                let value =
                    radial_value(
                        (
                            x as f32
                                + 0.5
                        )
                            / 32.0,

                        (
                            y as f32
                                + 0.5
                        )
                            / 32.0,

                        12345,
                        &layout,
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

        let layout=RadialLayout::new(144);


        let first =
            radial_value(
                0.37,
                0.63,
                999,
                &layout,
            );


        let second =
            radial_value(
                0.37,
                0.63,
                999,
                &layout,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_output() {

        let layout=RadialLayout::new(144);


        let mut found_difference =
            false;


        for y in
            0..16
        {
            for x in
                0..16
            {
                let sample_x =
                    (
                        x as f32
                            + 0.5
                    )
                        / 16.0;


                let sample_y =
                    (
                        y as f32
                            + 0.5
                    )
                        / 16.0;


                let first =
                    radial_value(
                        sample_x,
                        sample_y,
                        1,
                        &layout,
                    );


                let second =
                    radial_value(
                        sample_x,
                        sample_y,
                        2,
                        &layout,
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
            "Different seeds produced identical values across every sampled coordinate"
        );
    }


    #[test]
    fn periodic_band_is_normalized() {

        for index in
            0..=100
        {
            let value =
                periodic_band(
                    index as f32
                        / 100.0,
                    0.46,
                    0.055,
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


    #[test]
    fn layout_uses_requested_primitive_count(){
        let l=RadialLayout::new(256);

        assert_eq!(
            l.radial_frequency,
            16.0
        );

        assert_eq!(
            l.angular_frequency,
            10.0
        );

        assert_eq!(
            l.angular_frequency.fract(),
            0.0
        );
    }

    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                Palette::Slate,
                12345,
                144,
            )
            .expect(
                "radial generation"
            );


        assert_eq!(
            texture.specification.family,
            TextureFamily::Radial
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

