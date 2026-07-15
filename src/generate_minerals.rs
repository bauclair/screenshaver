//! Procedural mineral texture generation.
//!
//! Minerals use Chebyshev Worley distance fields and a
//! deterministic facet engine to create hard-edged crystalline
//! rock structures.
//!
//! This implementation combines:
//!
//! - large Chebyshev crystal regions;
//! - multiple competing planes inside every major crystal;
//! - fine Chebyshev micro-facets;
//! - high-frequency mineral grain;
//! - narrow internal and inter-crystal fractures;
//! - stable simulated lighting;
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

/// Maximum displacement of each major feature point from the
/// center of its grid cell.
const FEATURE_JITTER: f32 =
    0.98;

/// Number of competing planes synthesized inside each major
/// crystal.
const FACET_COUNT: u32 =
    12;

/// Strength of the piecewise-planar facet surface.
const FACET_HEIGHT_STRENGTH: f32 =
    0.34;

/// Strength of the lighting assigned to the winning facet.
const FACET_LIGHT_STRENGTH: f32 =
    0.30;

/// Width of internal facet seams.
const FACET_SEAM_WIDTH: f32 =
    0.055;

/// Darkness of internal facet seams.
const FACET_SEAM_DARKENING: f32 =
    0.16;

/// Frequency of fine crystalline breakup.
const MICRO_CRYSTAL_FREQUENCY: f32 =
    29.0;

/// Jitter used by the fine Chebyshev crystal field.
const MICRO_FEATURE_JITTER: f32 =
    0.96;

/// Strength of fine angular crystal variation.
const MICRO_CRYSTAL_STRENGTH: f32 =
    0.105;

/// Strength of fine fracture lines.
const MICRO_FRACTURE_STRENGTH: f32 =
    0.085;

/// Lower edge of the per-crystal growth transition.
const GROWTH_MASK_LOW: f32 =
    0.34;

/// Upper edge of the per-crystal growth transition.
const GROWTH_MASK_HIGH: f32 =
    0.72;

/// Minimum amount of fine structure retained even in crystals
/// with little secondary growth.
const MINIMUM_GROWTH_DETAIL: f32 =
    0.08;

/// Frequency of the first mineral-grain layer.
const GRAIN_FREQUENCY_1: f32 =
    83.0;

/// Frequency of the second mineral-grain layer.
const GRAIN_FREQUENCY_2: f32 =
    173.0;

/// Strength of high-frequency mineral grain.
const GRAIN_STRENGTH: f32 =
    0.075;

/// Stable per-crystal brightness variation.
const CRYSTAL_VARIATION_STRENGTH: f32 =
    0.16;

/// Contribution of the broad Chebyshev F1 structure.
const LARGE_STRUCTURE_STRENGTH: f32 =
    0.22;

/// Darkness of boundaries between major crystals.
const MAJOR_FRACTURE_DARKENING: f32 =
    0.13;

/// Sharpness of boundaries between major crystals.
const MAJOR_FRACTURE_SHARPNESS: f32 =
    2.8;

/// Minimum visibility retained for every major boundary.
const EDGE_VISIBILITY_MINIMUM: f32 =
    0.04;

/// Exponent used to bias most boundaries toward low visibility.
///
/// Values greater than 1.0 make strong boundaries uncommon.
const EDGE_VISIBILITY_BIAS: f32 =
    2.35;

/// Fixed simulated light direction.
const LIGHT_X: f32 =
    -0.58;

const LIGHT_Y: f32 =
    -0.81;

/// Final output contrast.
const CONTRAST: f32 =
    1.17;

/// Final brightness adjustment.
const BRIGHTNESS: f32 =
    0.015;


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
            FEATURE_JITTER,
        );


    let local_x =
        sample_x
            - crystal.feature_x;


    let local_y =
        sample_y
            - crystal.feature_y;


    //---------------------------------------------------------
    // The facet engine creates multiple competing planes inside
    // every major crystal. The largest plane score wins, which
    // creates abrupt, deterministic changes between flat faces.
    //---------------------------------------------------------

    let facet =
        facet_field(
            local_x,
            local_y,
            crystal.cell_x,
            crystal.cell_y,
            seed,
        );


    //---------------------------------------------------------
    // Broad Chebyshev F1 structure keeps the major crystal
    // masses visible beneath the internal facet system.
    //---------------------------------------------------------

    let large_structure =
        (
            1.0
                - crystal.first_distance
                    / 1.18
        )
        .clamp(
            0.0,
            1.0,
        );


    //---------------------------------------------------------
    // F2 - F1 approaches zero near major-crystal boundaries.
    // It is sharpened into restrained fracture darkening rather
    // than a heavy closed outline.
    //---------------------------------------------------------

    let major_gap =
        (
            crystal.second_distance
                - crystal.first_distance
        )
        .clamp(
            0.0,
            1.0,
        );


    let major_fracture =
        (
            1.0
                - major_gap
                    * 3.0
        )
        .clamp(
            0.0,
            1.0,
        )
        .powf(
            MAJOR_FRACTURE_SHARPNESS
        );


    //---------------------------------------------------------
    // Give the relationship between the nearest and second-
    // nearest crystals its own deterministic visibility.
    //
    // Most boundaries become weak or nearly invisible. A small
    // minority remain prominent and read as major fractures.
    //---------------------------------------------------------

    let edge_visibility =
        edge_visibility(
            crystal.cell_x,
            crystal.cell_y,
            crystal.second_cell_x,
            crystal.second_cell_y,
            seed.wrapping_add(
                0xDB4F_0B91_75AE_2165
            ),
        );


    //---------------------------------------------------------
    // A much finer Chebyshev field breaks the major faces into
    // crystalline chips and creates high-frequency fracture
    // structure.
    //---------------------------------------------------------

    let micro =
        nearest_crystal(
            x * MICRO_CRYSTAL_FREQUENCY,
            y * MICRO_CRYSTAL_FREQUENCY,
            seed.wrapping_add(
                0xD1B5_4A32_D192_ED03
            ),
            MICRO_FEATURE_JITTER,
        );


    let micro_structure =
        (
            1.0
                - micro.first_distance
                    / 1.08
        )
        .clamp(
            0.0,
            1.0,
        );


    let micro_gap =
        (
            micro.second_distance
                - micro.first_distance
        )
        .clamp(
            0.0,
            1.0,
        );


    let micro_fracture =
        (
            1.0
                - micro_gap
                    * 4.2
        )
        .clamp(
            0.0,
            1.0,
        )
        .powf(
            3.2
        );


    //---------------------------------------------------------
    // Two high-frequency value-noise layers add mineral grain,
    // pits, and inclusions without softening the crystal planes.
    //---------------------------------------------------------

    let grain_1 =
        value_noise(
            x * GRAIN_FREQUENCY_1,
            y * GRAIN_FREQUENCY_1,
            seed.wrapping_add(
                0x94D0_49BB_1331_11EB
            ),
        );


    let grain_2 =
        value_noise(
            x * GRAIN_FREQUENCY_2,
            y * GRAIN_FREQUENCY_2,
            seed.wrapping_add(
                0xBF58_476D_1CE4_E5B9
            ),
        );


    let grain =
        (
            grain_1
                * 0.68
            + grain_2
                * 0.32
            - 0.5
        )
        * GRAIN_STRENGTH;


    let crystal_variation =
        crystal_attribute(
            crystal.cell_x,
            crystal.cell_y,
            seed.wrapping_add(
                0x9E37_79B9_7F4A_7C15
            ),
        );


    //---------------------------------------------------------
    // Give each major crystal its own deterministic secondary-
    // growth characteristic.
    //
    // Low-growth crystals retain broad, relatively clean faces.
    // High-growth crystals reveal more micro-crystals and fine
    // fractures. This prevents the same detail density from
    // appearing everywhere.
    //---------------------------------------------------------

    let crystal_growth =
        crystal_attribute(
            crystal.cell_x,
            crystal.cell_y,
            seed.wrapping_add(
                0x8CB9_2BA7_2F3D_8DD7
            ),
        );


    let growth_mask =
        MINIMUM_GROWTH_DETAIL
            + (
                1.0
                    - MINIMUM_GROWTH_DETAIL
            )
            * smoothstep(
                GROWTH_MASK_LOW,
                GROWTH_MASK_HIGH,
                crystal_growth,
            );


    let value =
        0.48
            + facet.height
                * FACET_HEIGHT_STRENGTH
            + (
                facet.light
                    - 0.5
            )
                * FACET_LIGHT_STRENGTH
            + large_structure
                * LARGE_STRUCTURE_STRENGTH
            + (
                micro_structure
                    - 0.5
            )
                * MICRO_CRYSTAL_STRENGTH
                * growth_mask
            + grain
            + (
                crystal_variation
                    - 0.5
            )
                * CRYSTAL_VARIATION_STRENGTH
            - facet.seam
                * FACET_SEAM_DARKENING
            - major_fracture
                * MAJOR_FRACTURE_DARKENING
                * edge_visibility
            - micro_fracture
                * MICRO_FRACTURE_STRENGTH
                * growth_mask;


    apply_tone_curve(
        value.clamp(
            0.0,
            1.0,
        )
    )
}


// ============================================================
// Facet engine
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct FacetSample {
    height: f32,
    light: f32,
    seam: f32,
}


/// Synthesize several stable planar faces inside one major
/// crystal.
///
/// Every plane receives a deterministic angle, offset, slope,
/// and light response derived from the crystal coordinates and
/// global seed. The highest plane score wins at each pixel.
///
/// The difference between the highest and second-highest scores
/// becomes an internal facet-seam mask.
fn facet_field(
    local_x: f32,
    local_y: f32,
    cell_x: i32,
    cell_y: i32,
    seed: u64,
) -> FacetSample {

    let mut best_score =
        -f32::MAX;


    let mut second_score =
        -f32::MAX;


    let mut winning_light =
        0.5_f32;


    for facet_index in
        0..FACET_COUNT
    {
        let facet_seed =
            seed
                .wrapping_add(
                    0xA24B_AED4_963E_E407
                )
                .wrapping_add(
                    (
                        facet_index as u64
                    )
                    .wrapping_mul(
                        0x9E37_79B9_7F4A_7C15
                    )
                );


        let angle =
            crystal_attribute(
                cell_x,
                cell_y,
                facet_seed,
            )
            * std::f32::consts::TAU;


        let normal_x =
            angle.cos();


        let normal_y =
            angle.sin();


        let offset =
            (
                crystal_attribute(
                    cell_x,
                    cell_y,
                    facet_seed.wrapping_add(
                        0x94D0_49BB_1331_11EB
                    ),
                )
                - 0.5
            )
            * 0.62;


        let slope =
            0.72
                + crystal_attribute(
                    cell_x,
                    cell_y,
                    facet_seed.wrapping_add(
                        0xBF58_476D_1CE4_E5B9
                    ),
                )
                * 0.72;


        let bias =
            (
                crystal_attribute(
                    cell_x,
                    cell_y,
                    facet_seed.wrapping_add(
                        0xC2B2_AE3D_27D4_EB4F
                    ),
                )
                - 0.5
            )
            * 0.18;


        let score =
            (
                local_x
                    * normal_x
                + local_y
                    * normal_y
                - offset
            )
            * slope
            + bias;


        let light =
            (
                normal_x
                    * LIGHT_X
                + normal_y
                    * LIGHT_Y
            )
            * 0.5
            + 0.5;


        if score
            > best_score
        {
            second_score =
                best_score;


            best_score =
                score;


            winning_light =
                light;

        } else if score
            > second_score
        {
            second_score =
                score;
        }
    }


    let separation =
        (
            best_score
                - second_score
        )
        .max(
            0.0
        );


    let seam =
        1.0
            - smoothstep(
                0.0,
                FACET_SEAM_WIDTH,
                separation,
            );


    FacetSample {
        height:
            (
                best_score
                    * 0.5
                + 0.5
            )
            .clamp(
                0.0,
                1.0,
            ),

        light:
            winning_light.clamp(
                0.0,
                1.0,
            ),

        seam:
            seam.clamp(
                0.0,
                1.0,
            ),
    }
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
    second_cell_x: i32,
    second_cell_y: i32,
    feature_x: f32,
    feature_y: f32,
    first_distance: f32,
    second_distance: f32,
}


fn nearest_crystal(
    x: f32,
    y: f32,
    seed: u64,
    jitter: f32,
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


    let mut second_cell_x =
        0_i32;


    let mut second_cell_y =
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
                    * jitter;


            let feature_y =
                cell_y as f32
                    + 0.5
                    + (
                        feature.1
                            - 0.5
                    )
                    * jitter;


            let delta_x =
                feature_x
                    - x;


            let delta_y =
                feature_y
                    - y;


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


                second_cell_x =
                    nearest_cell_x;


                second_cell_y =
                    nearest_cell_y;


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


                second_cell_x =
                    cell_x;


                second_cell_y =
                    cell_y;
            }
        }
    }


    CrystalSample {
        cell_x:
            nearest_cell_x,

        cell_y:
            nearest_cell_y,

        second_cell_x,

        second_cell_y,

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
// High-frequency grain
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
        crystal_attribute(
            x0,
            y0,
            seed,
        );


    let value_10 =
        crystal_attribute(
            x1,
            y0,
            seed,
        );


    let value_01 =
        crystal_attribute(
            x0,
            y1,
            seed,
        );


    let value_11 =
        crystal_attribute(
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
// Edge hierarchy
// ============================================================

/// Return a deterministic and symmetric visibility value for
/// the boundary shared by two crystal cells.
///
/// Canonical ordering guarantees that A↔B and B↔A produce the
/// same result.
fn edge_visibility(
    first_x: i32,
    first_y: i32,
    second_x: i32,
    second_y: i32,
    seed: u64,
) -> f32 {

    let first =
        (
            first_x,
            first_y,
        );


    let second =
        (
            second_x,
            second_y,
        );


    let (
        low,
        high,
    ) =
        if first
            <= second
        {
            (
                first,
                second,
            )

        } else {

            (
                second,
                first,
            )
        };


    let low_hash =
        hash_coordinates(
            low.0,
            low.1,
            seed,
        );


    let high_hash =
        hash_coordinates(
            high.0,
            high.1,
            seed.wrapping_add(
                0xA24B_AED4_963E_E407
            ),
        );


    let combined =
        mix_u64(
            low_hash
                ^ high_hash.rotate_left(
                    29
                )
                ^ seed.rotate_right(
                    17
                )
        );


    let raw =
        hash_to_unit_float(
            combined
        );


    EDGE_VISIBILITY_MINIMUM
        + (
            1.0
                - EDGE_VISIBILITY_MINIMUM
        )
        * raw.powf(
            EDGE_VISIBILITY_BIAS
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
                    mineral_value(
                        sample_x,
                        sample_y,
                        1,
                    );


                let second =
                    mineral_value(
                        sample_x,
                        sample_y,
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
            "Different seeds produced identical values across every sampled coordinate"
        );
    }


    #[test]
    fn facet_field_is_deterministic() {

        let first =
            facet_field(
                0.13,
                -0.21,
                3,
                -4,
                999,
            );


        let second =
            facet_field(
                0.13,
                -0.21,
                3,
                -4,
                999,
            );


        assert_eq!(
            first.height,
            second.height
        );


        assert_eq!(
            first.light,
            second.light
        );


        assert_eq!(
            first.seam,
            second.seam
        );
    }


    #[test]
    fn crystal_growth_attribute_is_deterministic() {

        let first =
            crystal_attribute(
                4,
                -3,
                777_u64.wrapping_add(
                    0x8CB9_2BA7_2F3D_8DD7
                ),
            );


        let second =
            crystal_attribute(
                4,
                -3,
                777_u64.wrapping_add(
                    0x8CB9_2BA7_2F3D_8DD7
                ),
            );


        assert_eq!(
            first,
            second
        );
    }


    #[test]
    fn edge_visibility_is_symmetric() {

        let forward =
            edge_visibility(
                3,
                -2,
                8,
                5,
                12345,
            );


        let reverse =
            edge_visibility(
                8,
                5,
                3,
                -2,
                12345,
            );


        assert_eq!(
            forward,
            reverse
        );
    }


    #[test]
    fn edge_visibility_is_normalized() {

        let visibility =
            edge_visibility(
                -7,
                11,
                4,
                -9,
                67890,
            );


        assert!(
            (
                0.0..=1.0
            )
            .contains(
                &visibility
            )
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

