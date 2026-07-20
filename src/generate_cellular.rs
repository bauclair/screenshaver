//! Procedural cellular texture generation.
//!
//! Cellular textures are produced from Worley-style feature
//! points. Each pixel measures its distance to nearby feature
//! points, producing organic cells and boundaries.
//!
//! This module is intentionally self-contained so its noise
//! behavior can evolve independently from the other texture
//! families.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Cellular-generation parameters
// ============================================================

/// Density and geometry derived from the requested primitive count.
#[derive(Clone, Copy, Debug)]
struct CellularLayout {
    cell_frequency: f32,
}

impl CellularLayout {
    fn new(
        requested_primitive_count: usize,
    ) -> Self {

        let primitive_count =
            requested_primitive_count
                .max(
                    1
                );


        // One feature point is generated per lattice cell.  A square
        // texture therefore uses the square root of the requested
        // primitive count as its frequency along each dimension.
        let cell_frequency =
            (
                primitive_count as f32
            )
            .sqrt()
            .max(
                1.0
            );


        Self {
            cell_frequency,
        }
    }
}

/// Amount by which each feature point may move away from the
/// center of its grid cell.
const FEATURE_JITTER: f32 =
    0.92;

/// Strength of low-frequency coordinate distortion.
const DOMAIN_WARP_STRENGTH: f32 =
    0.20;

/// Frequency of the low-frequency domain-warp field.
const WARP_FREQUENCY: f32 =
    1.65;

/// Number of value-noise octaves used for coordinate warping.
const WARP_OCTAVES: u32 =
    4;

/// Frequency increase between warp octaves.
const WARP_FREQUENCY_MULTIPLIER: f32 =
    2.0;

/// Amplitude reduction between warp octaves.
const WARP_AMPLITUDE_MULTIPLIER: f32 =
    0.52;

/// Sharpness of cell boundaries.
///
/// Larger values produce thinner, more distinct borders.
const BOUNDARY_SHARPNESS: f32 =
    2.2;

/// Contribution of the nearest-feature distance.
const INTERIOR_WEIGHT: f32 =
    0.68;

/// Contribution of the cell-boundary field.
const BOUNDARY_WEIGHT: f32 =
    0.24;

/// Contribution of subtle interior surface variation.
const SURFACE_WEIGHT: f32 =
    0.08;

/// Final contrast adjustment.
const CONTRAST: f32 =
    1.04;

/// Final brightness adjustment.
const BRIGHTNESS: f32 =
    0.01;


// ============================================================
// Public generator
// ============================================================

pub fn generate(
    palette: Palette,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    let layout =
        CellularLayout::new(
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
                    "Cellular texture buffer size overflow"
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
                cellular_value(
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
        TextureFamily::Cellular,
        palette,
        seed,
    )
}


// ============================================================
// Cellular field
// ============================================================

fn cellular_value(
    x: f32,
    y: f32,
    seed: u64,
    layout: &CellularLayout,
) -> f32 {

    //---------------------------------------------------------
    // Low-frequency domain warping prevents the cells from
    // appearing as a perfectly regular Voronoi grid.
    //---------------------------------------------------------

    let warp_x =
        fractal_noise(
            x * WARP_FREQUENCY + 17.3,
            y * WARP_FREQUENCY - 9.7,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        )
        - 0.5;


    let warp_y =
        fractal_noise(
            x * WARP_FREQUENCY - 21.8,
            y * WARP_FREQUENCY + 14.4,
            seed.wrapping_add(
                0x9E37_79B9_7F4A_7C15
            ),
        )
        - 0.5;


    let warped_x =
        x
            + warp_x
                * DOMAIN_WARP_STRENGTH;


    let warped_y =
        y
            + warp_y
                * DOMAIN_WARP_STRENGTH;


    let sample_x =
        warped_x
            * layout.cell_frequency;


    let sample_y =
        warped_y
            * layout.cell_frequency;


    let distances =
        nearest_feature_distances(
            sample_x,
            sample_y,
            seed,
        );


    //---------------------------------------------------------
    // F1 is the distance to the nearest feature point.
    //
    // F2 - F1 becomes small near boundaries and larger inside
    // cells, making it useful as a boundary-distance field.
    //---------------------------------------------------------

    let nearest =
        distances.0;


    let second_nearest =
        distances.1;


    let interior =
        (
            1.0
                - nearest
                    / 1.15
        )
        .clamp(
            0.0,
            1.0,
        );


    let boundary_distance =
        (
            second_nearest
                - nearest
        )
        .clamp(
            0.0,
            1.0,
        );


    let boundary =
        (
            1.0
                - boundary_distance
                    * 3.0
        )
        .clamp(
            0.0,
            1.0,
        )
        .powf(
            BOUNDARY_SHARPNESS
        );


    //---------------------------------------------------------
    // Add gentle variation inside the cells so they do not
    // appear as flat, uniformly filled polygons.
    //---------------------------------------------------------

    let surface =
        fractal_noise(
            warped_x * 3.1 + 33.0,
            warped_y * 3.1 - 27.0,
            seed.wrapping_add(
                0xD1B5_4A32_D192_ED03
            ),
        );


    //---------------------------------------------------------
    // Keep the cell interiors dominant while allowing the
    // boundaries to remain visible enough for refraction and
    // distortion shaders.
    //---------------------------------------------------------

    let combined =
        interior
            * INTERIOR_WEIGHT
        + boundary
            * BOUNDARY_WEIGHT
        + surface
            * SURFACE_WEIGHT;


    apply_tone_curve(
        combined
    )
}


// ============================================================
// Worley / Voronoi feature distances
// ============================================================

fn nearest_feature_distances(
    x: f32,
    y: f32,
    seed: u64,
) -> (
    f32,
    f32,
) {

    let base_x =
        x.floor() as i32;


    let base_y =
        y.floor() as i32;


    let mut nearest =
        f32::MAX;


    let mut second_nearest =
        f32::MAX;


    //---------------------------------------------------------
    // Searching the surrounding 3x3 grid is sufficient because
    // every cell contains one feature point.
    //---------------------------------------------------------

    for offset_y in
        -1..=1
    {
        for offset_x in
            -1..=1
        {
            let cell_x =
                base_x
                    + offset_x;


            let cell_y =
                base_y
                    + offset_y;


            let feature =
                feature_point(
                    cell_x,
                    cell_y,
                    seed,
                );


            let feature_x =
                cell_x as f32
                    + 0.5
                    + (
                        feature.0 - 0.5
                    )
                    * FEATURE_JITTER;


            let feature_y =
                cell_y as f32
                    + 0.5
                    + (
                        feature.1 - 0.5
                    )
                    * FEATURE_JITTER;


            let delta_x =
                feature_x
                    - x;


            let delta_y =
                feature_y
                    - y;


            let distance =
                (
                    delta_x
                        * delta_x
                    + delta_y
                        * delta_y
                )
                .sqrt();


            if distance
                < nearest
            {
                second_nearest =
                    nearest;


                nearest =
                    distance;

            } else if distance
                < second_nearest
            {
                second_nearest =
                    distance;
            }
        }
    }


    (
        nearest,
        second_nearest,
    )
}


fn feature_point(
    cell_x: i32,
    cell_y: i32,
    seed: u64,
) -> (
    f32,
    f32,
) {

    let first =
        hash_coordinates(
            cell_x,
            cell_y,
            seed,
        );


    let second =
        hash_coordinates(
            cell_x,
            cell_y,
            seed.wrapping_add(
                0x94D0_49BB_1331_11EB
            ),
        );


    (
        hash_to_unit_float(
            first
        ),
        hash_to_unit_float(
            second
        ),
    )
}


// ============================================================
// Low-frequency warp noise
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

    hash_to_unit_float(
        hash_coordinates(
            x,
            y,
            seed,
        )
    )
}


// ============================================================
// Hashing
// ============================================================

fn hash_coordinates(
    x: i32,
    y: i32,
    seed: u64,
) -> u64 {

    let x_bits =
        x as i64
            as u64;


    let y_bits =
        y as i64
            as u64;


    let value =
        seed
            ^ x_bits.wrapping_mul(
                0x9E37_79B1_85EB_CA87
            )
            ^ y_bits.wrapping_mul(
                0xC2B2_AE3D_27D4_EB4F
            );


    mix_u64(
        value
    )
}


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
// Tone mapping and interpolation
// ============================================================

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
    fn cellular_value_is_normalized() {

        let layout =
            CellularLayout::new(
                144
            );


        for y in
            0..16
        {
            for x in
                0..16
            {
                let value =
                    cellular_value(
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
            CellularLayout::new(
                144
            );


        let first =
            cellular_value(
                0.37,
                0.63,
                999,
                &layout,
            );


        let second =
            cellular_value(
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

        let layout =
            CellularLayout::new(
                144
            );


        let first =
            cellular_value(
                0.37,
                0.63,
                1,
                &layout,
            );


        let second =
            cellular_value(
                0.37,
                0.63,
                2,
                &layout,
            );


        assert_ne!(
            first,
            second
        );
    }


    #[test]
    fn feature_points_are_normalized() {

        let feature =
            feature_point(
                3,
                -2,
                12345,
            );


        assert!(
            (
                0.0..=1.0
            )
            .contains(
                &feature.0
            )
        );


        assert!(
            (
                0.0..=1.0
            )
            .contains(
                &feature.1
            )
        );
    }


    #[test]
    fn layout_uses_requested_primitive_count() {

        let layout =
            CellularLayout::new(
                256
            );


        assert_eq!(
            layout.cell_frequency,
            16.0
        );
    }


    #[test]
    fn different_primitive_counts_change_output() {

        let sparse_layout =
            CellularLayout::new(
                16
            );


        let dense_layout =
            CellularLayout::new(
                256
            );


        let sparse =
            cellular_value(
                0.37,
                0.63,
                999,
                &sparse_layout,
            );


        let dense =
            cellular_value(
                0.37,
                0.63,
                999,
                &dense_layout,
            );


        assert_ne!(
            sparse,
            dense
        );
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                Palette::Lichen,
                12345,
                144,
            )
            .expect(
                "cellular generation"
            );


        assert_eq!(
            texture.family,
            TextureFamily::Cellular
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

