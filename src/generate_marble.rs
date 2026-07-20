//! Procedural marble texture generation.
//!
//! Marble is created by distorting a set of smooth sinusoidal
//! veins with layered value noise. The generator produces a
//! normalized scalar field and delegates color mapping to
//! palettes.rs.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Marble-generation parameters
// ============================================================

/// Number of noise octaves used for turbulence and surface
/// detail.
const OCTAVES: u32 =
    8;

/// Starting frequency for layered value noise.
const BASE_NOISE_FREQUENCY: f32 =
    1.55;

/// Frequency increase between octaves.
const FREQUENCY_MULTIPLIER: f32 =
    2.0;

/// Amplitude reduction between octaves.
const AMPLITUDE_MULTIPLIER: f32 =
    0.52;

/// Overall direction of the marble flow.
const DIRECTION_DEGREES: f32 =
    55.0;

/// Vein density derived from the requested primitive count.
#[derive(Clone, Copy, Debug)]
struct MarbleLayout {
    vein_scale: f32,
}

impl MarbleLayout {
    fn new(
        requested_primitive_count: usize,
    ) -> Self {

        let primitive_count =
            requested_primitive_count
                .max(
                    1
                );


        // Preserve the original visual density at the standard request
        // of 144 primitives, while scaling broad vein density with the
        // square root of the requested count.
        let reference_frequency =
            144.0_f32.sqrt();


        let vein_scale =
            2.20
                * (
                    primitive_count as f32
                )
                .sqrt()
                / reference_frequency;


        Self {
            vein_scale:
                vein_scale.max(
                    0.10
                ),
        }
    }
}

/// Strength of coordinate distortion.
const DOMAIN_WARP_STRENGTH: f32 =
    0.52;

/// Sharpness of the vein boundaries.
const VEIN_SHARPNESS: f32 =
    3.0;

/// Final contrast adjustment.
const CONTRAST: f32 =
    0.94;

/// Final brightness adjustment.
const BRIGHTNESS: f32 =
    0.06;


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    let layout =
        MarbleLayout::new(
            requested_primitive_count
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
                    "Marble texture buffer size overflow"
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
                marble_value(
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
        TextureFamily::Marble,
        palette,
        seed,
    )
}


// ============================================================
// Marble field
// ============================================================

fn marble_value(
    x: f32,
    y: f32,
    seed: u64,
    layout: &MarbleLayout,
) -> f32 {

    let angle =
        DIRECTION_DEGREES
            .to_radians();


    let direction_x =
        angle.cos();


    let direction_y =
        angle.sin();


    //---------------------------------------------------------
    // Rotate the coordinate system so the marble has a broad
    // preferred direction without creating regular stripes.
    //---------------------------------------------------------

    let directional_x =
        x * direction_x
            + y * direction_y;


    let directional_y =
        -x * direction_y
            + y * direction_x;


    //---------------------------------------------------------
    // Generate two independent low-frequency fields and use
    // them to distort the sampling coordinates.
    //---------------------------------------------------------

    let warp_x =
        fractal_noise(
            x * 0.85 + 17.31,
            y * 0.85 - 9.47,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        )
        - 0.5;


    let warp_y =
        fractal_noise(
            x * 0.85 - 23.18,
            y * 0.85 + 14.62,
            seed.wrapping_add(
                0x9E37_79B9_7F4A_7C15
            ),
        )
        - 0.5;


    let warped_x =
        directional_x
            + warp_x
                * DOMAIN_WARP_STRENGTH;


    let warped_y =
        directional_y
            + warp_y
                * DOMAIN_WARP_STRENGTH;


    //---------------------------------------------------------
    // Build broad, irregular ridges from warped fractal noise.
    //
    // This replaces the previous sine-wave implementation,
    // which produced regular diagonal bands.
    //---------------------------------------------------------

    let broad_noise =
        fractal_noise(
            warped_x
                * layout.vein_scale,
            warped_y
                * layout.vein_scale
                * 0.62,
            seed.wrapping_add(
                0xD1B5_4A32_D192_ED03
            ),
        );


    let broad_ridge =
        ridged_value(
            broad_noise
        );


    let broad_veins =
        smoothstep(
            0.18,
            0.88,
            broad_ridge,
        )
        .powf(
            VEIN_SHARPNESS
        );


    //---------------------------------------------------------
    // Add smaller irregular structures that soften and break
    // up the broad channels.
    //---------------------------------------------------------

    let secondary_noise =
        fractal_noise(
            warped_x * 4.25 + 41.0,
            warped_y * 3.10 - 27.0,
            seed.wrapping_add(
                0x94D0_49BB_1331_11EB
            ),
        );


    let secondary_veins =
        ridged_value(
            secondary_noise
        )
        .powf(
            4.0
        );


    //---------------------------------------------------------
    // Break up the vein opacity so the channels do not appear
    // as uninterrupted ribbons.
    //---------------------------------------------------------

    let breakup =
        fractal_noise(
            warped_x * 1.35 + 73.0,
            warped_y * 1.35 - 51.0,
            seed.wrapping_add(
                0xBF58_476D_1CE4_E5B9
            ),
        );


    let vein_mask =
        (
            broad_veins
                * 0.82
            + secondary_veins
                * 0.18
        )
        * (
            0.68
                + breakup
                    * 0.32
        );


    //---------------------------------------------------------
    // Generate subtle texture within the pale stone material.
    //---------------------------------------------------------

    let surface_detail =
        fractal_noise(
            warped_x * 2.15 + 11.0,
            warped_y * 2.15 + 29.0,
            seed.wrapping_add(
                0xC2B2_AE3D_27D4_EB4F
            ),
        );


    let stone_base =
        0.82
            + surface_detail
                * 0.16;


    let value =
        stone_base
            - vein_mask
                * 0.56;


    apply_tone_curve(
        value.clamp(
            0.0,
            1.0,
        )
    )
}

fn ridged_value(
    value: f32,
) -> f32 {

    (
        1.0
            - (
                value * 2.0
                    - 1.0
            )
            .abs()
    )
    .clamp(
        0.0,
        1.0,
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
// Layered value noise
// ============================================================

fn fractal_noise(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let mut frequency =
        BASE_NOISE_FREQUENCY;


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
            FREQUENCY_MULTIPLIER;


        amplitude *=
            AMPLITUDE_MULTIPLIER;
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


// ============================================================
// Value noise
// ============================================================

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
    fn layout_preserves_reference_density() {

        let layout =
            MarbleLayout::new(
                144
            );


        assert!(
            (
                layout.vein_scale
                    - 2.20
            )
            .abs()
                <= f32::EPSILON
        );
    }


    #[test]
    fn larger_primitive_counts_increase_vein_density() {

        let sparse =
            MarbleLayout::new(
                16
            );


        let dense =
            MarbleLayout::new(
                256
            );


        assert!(
            dense.vein_scale
                > sparse.vein_scale
        );
    }


    #[test]
    fn different_primitive_counts_change_output() {

        let sparse_layout =
            MarbleLayout::new(
                16
            );


        let dense_layout =
            MarbleLayout::new(
                256
            );


        let sparse =
            marble_value(
                0.31,
                0.67,
                999,
                &sparse_layout,
            );


        let dense =
            marble_value(
                0.31,
                0.67,
                999,
                &dense_layout,
            );


        assert_ne!(
            sparse,
            dense
        );
    }


    #[test]
    fn marble_value_is_normalized() {

        let layout =
            MarbleLayout::new(
                144
            );



        for y in
            0..16
        {
            for x in
                0..16
            {
                let value =
                    marble_value(
                        x as f32
                            / 16.0,
                        y as f32
                            / 16.0,
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

        let layout =
            MarbleLayout::new(
                144
            );



        let first =
            marble_value(
                0.31,
                0.67,
                999,
                &layout,
            );


        let second =
            marble_value(
                0.31,
                0.67,
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

        let layout =
            MarbleLayout::new(
                144
            );



        let first =
            marble_value(
                0.31,
                0.67,
                1,
                &layout,
            );


        let second =
            marble_value(
                0.31,
                0.67,
                2,
                &layout,
            );


        assert_ne!(
            first,
            second
        );
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                Palette::Sandstone,
                12345,
                144,
            )
            .expect(
                "marble generation"
            );


        assert_eq!(
            texture.family,
            TextureFamily::Marble
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

