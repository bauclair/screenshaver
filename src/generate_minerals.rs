//! Procedural mineral texture generation.
//!
//! Minerals use Chebyshev Worley distance fields to create
//! hard-edged crystalline planes that remain visually distinct
//! from the softer Euclidean cellular texture family.
//!
//! This implementation combines:
//!
//! - Chebyshev F1 for large angular crystal structure;
//! - Chebyshev F2 - F1 for fractures and breakup;
//! - stable per-crystal facet lighting;
//! - subtle planar variation;
//! - palette mapping performed by palettes.rs.

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::Palette;


// ============================================================
// Mineral-generation parameters
// ============================================================

/// Approximate number of major crystal regions across each
/// texture dimension.
const CRYSTAL_FREQUENCY: f32 =
    5.5;

/// Maximum displacement of each feature point from the center
/// of its grid cell.
const FEATURE_JITTER: f32 =
    0.98;

/// Strength of stable per-crystal brightness variation.
const CRYSTAL_VARIATION_STRENGTH: f32 =
    0.22;

/// Strength of simulated directional lighting.
const ORIENTATION_LIGHT_STRENGTH: f32 =
    0.34;

/// Strength of the local planar gradient within each crystal.
const FACET_SLOPE_STRENGTH: f32 =
    0.46;

/// Contribution of the large Chebyshev F1 structure.
const F1_WEIGHT: f32 =
    0.72;

/// Contribution of the Chebyshev F2 - F1 fracture field.
const FRACTURE_WEIGHT: f32 =
    0.28;

/// Sharpness applied to the fracture field.
const FRACTURE_SHARPNESS: f32 =
    2.35;

/// Fixed simulated light direction.
const LIGHT_X: f32 =
    -0.58;

const LIGHT_Y: f32 =
    -0.81;

/// Final output contrast.
const CONTRAST: f32 =
    1.14;

/// Final brightness adjustment.
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
                    "Mineral texture buffer size overflow"
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
                mineral_value(
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
        TextureFamily::Minerals,
        palette,
        seed,
    )
}


// ============================================================
// Mineral field
// ============================================================

fn mineral_value(
    x: f32,
    y: f32,
    seed: u64,
) -> f32 {

    let sample_x =
        x
            * CRYSTAL_FREQUENCY;


    let sample_y =
        y
            * CRYSTAL_FREQUENCY;


    let crystal =
        nearest_crystal(
            sample_x,
            sample_y,
            seed,
        );


    let local_x =
        sample_x
            - crystal.feature_x;


    let local_y =
        sample_y
            - crystal.feature_y;


    //---------------------------------------------------------
    // Each crystal receives a stable orientation. This is used
    // for both a directional-light response and a local planar
    // brightness gradient.
    //---------------------------------------------------------

    let facet_angle =
        crystal_attribute(
            crystal.cell_x,
            crystal.cell_y,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        )
        * std::f32::consts::TAU;


    let normal_x =
        facet_angle.cos();


    let normal_y =
        facet_angle.sin();


    let orientation_light =
        (
            normal_x
                * LIGHT_X
            + normal_y
                * LIGHT_Y
        )
        * 0.5
        + 0.5;


    let planar_gradient =
        (
            local_x
                * normal_x
            + local_y
                * normal_y
        )
        * FACET_SLOPE_STRENGTH;


    //---------------------------------------------------------
    // Chebyshev F1 produces broad, blocky angular regions.
    //
    // Normalize it to a useful brightness field. Near a feature
    // point the value is brighter; farther away it becomes
    // darker, creating hard planar crystal structure.
    //---------------------------------------------------------

    let f1_structure =
        (
            1.0
                - crystal.first_distance
                    / 1.15
        )
        .clamp(
            0.0,
            1.0,
        );


    //---------------------------------------------------------
    // F2 - F1 becomes small along cell boundaries.
    //
    // Invert and sharpen it to produce narrow crystalline
    // fracture structures without drawing heavy black outlines.
    //---------------------------------------------------------

    let distance_gap =
        (
            crystal.second_distance
                - crystal.first_distance
        )
        .clamp(
            0.0,
            1.0,
        );


    let fracture =
        (
            1.0
                - distance_gap
                    * 2.75
        )
        .clamp(
            0.0,
            1.0,
        )
        .powf(
            FRACTURE_SHARPNESS
        );


    //---------------------------------------------------------
    // Stable per-crystal variation keeps neighboring regions
    // from sharing the same baseline intensity.
    //---------------------------------------------------------

    let crystal_variation =
        crystal_attribute(
            crystal.cell_x,
            crystal.cell_y,
            seed.wrapping_add(
                0x9E37_79B9_7F4A_7C15
            ),
        );


    //---------------------------------------------------------
    // Build the final mineral intensity.
    //
    // Geometry comes primarily from Chebyshev F1. Fractures
    // darken and break up the structure. Lighting and planar
    // gradients create the impression of differently oriented
    // crystal faces.
    //---------------------------------------------------------

    let structure =
        f1_structure
            * F1_WEIGHT
        + (
            1.0
                - fracture
        )
            * FRACTURE_WEIGHT;


    let value =
        0.34
            + structure
                * 0.54
            + (
                orientation_light
                    - 0.5
            )
                * ORIENTATION_LIGHT_STRENGTH
            + planar_gradient
            + (
                crystal_variation
                    - 0.5
            )
                * CRYSTAL_VARIATION_STRENGTH
            - fracture
                * 0.18;


    apply_tone_curve(
        value.clamp(
            0.0,
            1.0,
        )
    )
}


// ============================================================
// Crystal lookup
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct CrystalSample {
    cell_x: i32,
    cell_y: i32,
    feature_x: f32,
    feature_y: f32,
    first_distance: f32,
    second_distance: f32,
}


fn nearest_crystal(
    x: f32,
    y: f32,
    seed: u64,
) -> CrystalSample {

    let base_x =
        x.floor() as i32;


    let base_y =
        y.floor() as i32;


    let mut nearest =
        f32::MAX;


    let mut second_nearest =
        f32::MAX;


    let mut nearest_cell_x =
        0_i32;


    let mut nearest_cell_y =
        0_i32;


    let mut nearest_feature_x =
        0.0_f32;


    let mut nearest_feature_y =
        0.0_f32;


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
                        feature.0
                            - 0.5
                    )
                    * FEATURE_JITTER;


            let feature_y =
                cell_y as f32
                    + 0.5
                    + (
                        feature.1
                            - 0.5
                    )
                    * FEATURE_JITTER;


            let delta_x =
                feature_x
                    - x;


            let delta_y =
                feature_y
                    - y;


            //-------------------------------------------------
            // Chebyshev distance is the defining characteristic
            // of this texture family.
            //-------------------------------------------------

            let distance =
                delta_x
                    .abs()
                    .max(
                        delta_y.abs()
                    );


            if distance
                < nearest
            {
                second_nearest =
                    nearest;


                nearest =
                    distance;


                nearest_cell_x =
                    cell_x;


                nearest_cell_y =
                    cell_y;


                nearest_feature_x =
                    feature_x;


                nearest_feature_y =
                    feature_y;

            } else if distance
                < second_nearest
            {
                second_nearest =
                    distance;
            }
        }
    }


    CrystalSample {
        cell_x:
            nearest_cell_x,

        cell_y:
            nearest_cell_y,

        feature_x:
            nearest_feature_x,

        feature_y:
            nearest_feature_y,

        first_distance:
            nearest,

        second_distance:
            second_nearest,
    }
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


fn crystal_attribute(
    cell_x: i32,
    cell_y: i32,
    seed: u64,
) -> f32 {

    hash_to_unit_float(
        hash_coordinates(
            cell_x,
            cell_y,
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
    fn mineral_value_is_normalized() {

        for y in
            0..16
        {
            for x in
                0..16
            {
                let value =
                    mineral_value(
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
            mineral_value(
                0.41,
                0.59,
                777,
            );


        let second =
            mineral_value(
                0.41,
                0.59,
                777,
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn different_seeds_change_output() {

        let first =
            mineral_value(
                0.41,
                0.59,
                1,
            );


        let second =
            mineral_value(
                0.41,
                0.59,
                2,
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
                Palette::Slate,
                12345,
            )
            .expect(
                "mineral generation"
            );


        assert_eq!(
            texture.family,
            TextureFamily::Minerals
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

